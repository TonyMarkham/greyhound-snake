namespace Greyhound.Step
{
    public enum StepActuatorType
    {
        // The MJCF <position> servo: force = kp·(ctrl − qpos) − kv·qvel.
        Position,
    }
}
