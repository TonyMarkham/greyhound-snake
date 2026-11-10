namespace Greyhound.MuJoCo
{
    // Schema enum jointtype: free, ball, slide, hinge (default hinge).
    // Slide and Hinge keep their original integer values; Free and Ball
    // are appended, because Unity serializes an enum by its integer.
    public enum JointType
    {
        Slide,
        Hinge,
        Free,
        Ball,
    }
}
