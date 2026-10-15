namespace Greyhound.Vision
{
    // Where the payload provisioning stands. Ready and Failed are
    // terminal for the current domain; Running means a download or
    // extraction is in flight and a reimport should be retried.
    internal enum VisionProvisionState
    {
        NotStarted,
        Running,
        Ready,
        Failed,
    }
}
