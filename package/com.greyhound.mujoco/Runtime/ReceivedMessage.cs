namespace Greyhound.MuJoCo
{
    // One whole WebSocket message as a StateSocket received it, queued for
    // the StateServer to handle on Unity's main thread: exactly one of Text
    // (a text message, e.g. a JSON joint state) and Binary (a binary
    // message, handed on untouched) is set, and Connection is the client to
    // reply to.
    internal sealed class ReceivedMessage
    {
        private ReceivedMessage(StateConnection connection, string text, byte[] binary)
        {
            Connection = connection;
            Text = text;
            Binary = binary;
        }

        public StateConnection Connection { get; }

        // Null for a binary message.
        public string Text { get; }

        // Null for a text message.
        public byte[] Binary { get; }

        public static ReceivedMessage FromText(StateConnection connection, string text)
        {
            return new ReceivedMessage(connection, text, null);
        }

        public static ReceivedMessage FromBinary(StateConnection connection, byte[] binary)
        {
            return new ReceivedMessage(connection, null, binary);
        }
    }
}
