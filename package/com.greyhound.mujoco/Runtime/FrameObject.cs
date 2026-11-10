namespace Greyhound.MuJoCo
{
    // Schema enum frameobj (mjcf-3.15.0.schema), in schema order: the
    // object kinds with a spatial frame, used by <framepos> objtype and
    // reftype. Keywords are the lower-case member names.
    public enum FrameObject
    {
        Body,
        XBody,
        Geom,
        Site,
        Camera,
    }
}
