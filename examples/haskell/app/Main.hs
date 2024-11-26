-- Simple Haskell program to subscribe for typed messages from Caryatid
{-# LANGUAGE OverloadedStrings #-}

import qualified Network.AMQP as AMQP
import Codec.CBOR.Decoding (Decoder, decodeInt, decodeString, decodeMapLen)
import Codec.CBOR.Read (deserialiseFromBytes)
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

    -- Declare the queue
    (queue, _, _) <- AMQP.declareQueue chan AMQP.newQueue {
        AMQP.queueName = "",
        AMQP.queueDurable = False
    }

    -- Bind the queue to the exchange with our topic
    AMQP.bindQueue chan queue "caryatid" "sample.test"

    -- Set up subscription to the queue
    _ <- AMQP.consumeMsgs chan queue AMQP.Ack
      $ \(msg, env) -> do
        let body = AMQP.msgBody msg
        case processMessage body of
            Left err -> putStrLn $ "Failed to decode message: " ++ err
            Right decoded -> putStrLn $ "Received: " ++ show decoded
        AMQP.ackEnv env

    -- Run until Enter pressed
    putStrLn "Press [Enter] to exit."
    void getLine

    -- Clean up
    AMQP.closeConnection conn
    putStrLn "Connection closed."
