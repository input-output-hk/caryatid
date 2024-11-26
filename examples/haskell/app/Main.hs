-- Simple Haskell program to subscribe for typed messages from Caryatid
{-# LANGUAGE OverloadedStrings #-}

import qualified Network.AMQP as AMQP
import Codec.CBOR.Decoding (Decoder, decodeInt, decodeString, decodeMapLen)
import Codec.CBOR.Read (deserialiseFromBytes)
import Codec.CBOR.Encoding
import Codec.CBOR.Write (toLazyByteString)
import qualified Data.ByteString.Lazy as BL
import qualified Data.Text as T
import Control.Monad (void)

-- Test structure (matching typed/src/messages.rs)
data Test = Test
    { dataField :: T.Text
    , numberField :: Int
    } deriving (Show)

-- Message enum (ditto, but simplified)
data Message
    = None
    | TestMessage Test
    | StringMessage T.Text
    deriving (Show)

-- Decoder for the Test struct
decodeTest :: Decoder s Test
decodeTest = do
    len <- decodeMapLen
    if len /= 2
      then fail "Test should have 2 fields"
      else do
        key1 <- decodeString
        value1 <- decodeString
        key2 <- decodeString
        value2 <- decodeInt
        if key1 == "data" && key2 == "number"
          then return $ Test value1 value2
          else fail "Unexpected keys in Test struct"

-- Decoder for the Message enum
decodeMessage :: Decoder s Message
decodeMessage = do
    len <- decodeMapLen
    if len /= 1
      then fail "Message should be a map with 1 key"
      else do
        tag <- decodeString -- Variant tag
        case tag of
          "None" -> return None
          "Test" -> TestMessage <$> decodeTest
          "String" -> StringMessage <$> decodeString
          _ -> fail $ "Unknown tag: " ++ show tag

-- Function to process incoming messages
processMessage :: BL.ByteString -> Either String Message
processMessage msg =
    case deserialiseFromBytes decodeMessage msg of
        Left err -> Left $ "Decoding error: " ++ show err
        Right (_, value) -> Right value

-- Encoder for the Test struct
encodeTest :: Test -> Encoding
encodeTest (Test df nf) =
    encodeMapLen 2 <>
    encodeString "data" <> encodeString df <>
    encodeString "number" <> encodeInt nf

-- Encoder for the Message enum
encodeMessage :: Message -> Encoding
encodeMessage None = encodeMapLen 1 <> encodeString "None" <> encodeNull
encodeMessage (TestMessage test) = encodeMapLen 1 <> encodeString "Test" <> encodeTest test
encodeMessage (StringMessage str) = encodeMapLen 1 <> encodeString "String" <> encodeString str

-- Serialize outgoing messages
serializeMessage :: Message -> BL.ByteString
serializeMessage = toLazyByteString . encodeMessage

-- Main
main :: IO ()
main = do
    -- Establish connection to RabbitMQ
    conn <- AMQP.openConnection "localhost" "/" "guest" "guest"
    chan <- AMQP.openChannel conn

    -- Declare exchange
    AMQP.declareExchange chan AMQP.newExchange {
        AMQP.exchangeName = "caryatid",
        AMQP.exchangeType = "topic",
        AMQP.exchangeDurable = False
    }

    -- Declare the input queue and bind it to sample.test
    (inputQueue, _, _) <- AMQP.declareQueue chan AMQP.newQueue {
        AMQP.queueName = "",
        AMQP.queueDurable = False
    }

    AMQP.bindQueue chan inputQueue "caryatid" "sample.test"

    -- Likewise the output queue, on sample.haskell
    (outputQueue, _, _) <- AMQP.declareQueue chan AMQP.newQueue {
        AMQP.queueName = "",
        AMQP.queueDurable = False
    }

    AMQP.bindQueue chan outputQueue "caryatid" "sample.haskell"

    -- Set up subscription to the input queue
    _ <- AMQP.consumeMsgs chan inputQueue AMQP.Ack
      $ \(msg, env) -> do
        let body = AMQP.msgBody msg
        case processMessage body of
            Left err -> putStrLn $ "Failed to decode message: " ++ err
            Right decoded -> do
              putStrLn $ "Received: " ++ show decoded

              case decoded of
                TestMessage (Test _ nf) -> do
                  -- Send response
                  let response = TestMessage (Test "Hello from Haskell!" nf)
                  let payload = serializeMessage response
                  _ <- AMQP.publishMsg chan "caryatid" "sample.haskell" AMQP.newMsg {
                    AMQP.msgBody = payload
                  }
                  putStrLn "Response sent."

                _ -> putStrLn "Unsupported message type"

        AMQP.ackEnv env

    -- Run until Enter pressed
    putStrLn "Press [Enter] to exit."
    void getLine

    -- Clean up
    AMQP.closeConnection conn
    putStrLn "Connection closed."
