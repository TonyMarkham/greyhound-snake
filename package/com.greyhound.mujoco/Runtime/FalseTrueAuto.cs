namespace Greyhound.MuJoCo
{
    // Schema enum FalseTrueAuto (mjcf-3.15.0.schema): false = 0,
    // true = 1, auto = 2. With compiler autolimits (default true), auto
    // turns the limit on exactly when the matching range is given.
    public enum FalseTrueAuto
    {
        False,
        True,
        Auto,
    }
}
