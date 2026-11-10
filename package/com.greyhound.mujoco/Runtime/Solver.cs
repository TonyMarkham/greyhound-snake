namespace Greyhound.MuJoCo
{
    // Schema enum solver (option/solver, default Newton). Keywords are
    // case-sensitive in MJCF: PGS, CG, Newton.
    public enum Solver
    {
        PGS,
        CG,
        Newton,
    }
}
