namespace Greyhound.Step
{
    public enum StepActuatorType
    {
        // The MJCF <position> servo: force = kp·(ctrl − qpos) − kv·qvel.
        // ctrl is a target position (m for a slide, rad for a hinge).
        Position,

        // The MJCF <velocity> servo: force = kv·(ctrl − qvel)
        // (gainprm kv, biasprm 0 0 −kv; xmlref 3.15.0, actuator/velocity).
        // ctrl is a target velocity (m/s for a slide, rad/s for a hinge).
        Velocity,
    }
}
