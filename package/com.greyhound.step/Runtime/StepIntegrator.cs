namespace Greyhound.Step
{
    // The MJCF option/integrator values at 3.15.0 (xmlref
    // #option-integrator). Not [Flags]: a model has exactly one
    // integrator, so the inspector draws a single-choice dropdown.
    // MuJoCo's own default is Euler; a StepJointSet defaults to
    // ImplicitFast, which the docs recommend for velocity actuators
    // and for most models (it integrates actuator damping such as a
    // <velocity> servo's kv implicitly, which Euler does not).
    public enum StepIntegrator
    {
        Euler,
        RK4,
        Implicit,
        ImplicitFast,
        Discrete,
    }
}
