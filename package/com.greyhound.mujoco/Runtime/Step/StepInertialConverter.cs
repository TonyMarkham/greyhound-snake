#if GREYHOUND_STEP
using Greyhound.Step;

namespace Greyhound.MuJoCo
{
    // Converts the STEP importer's exact mass properties for a part into
    // the MuJoCo <inertial> payload. Both sides use the same frame (the
    // part's Unity frame) and units (kg, meters, kg·m²), so every value
    // is copied as is:
    //   centreOfMass          -> pos
    //   inertiaTensorRotation -> quat
    //   massKilograms         -> mass (double narrowed to float)
    //   inertiaTensor         -> diaginertia (principal moments in the
    //                            frame inertiaTensorRotation rotates to)
    // The result uses the diaginertia + quat form, never fullinertia.
    //
    // Optional dependency: this file compiles only when
    // com.greyhound.step is installed. The runtime asmdef defines
    // GREYHOUND_STEP from that package's presence (versionDefines) and
    // references com.greyhound.step.runtime; without step the reference
    // is unresolved and this file is excluded, so the mujoco package
    // builds on its own. Callers wrap their use in #if GREYHOUND_STEP.
    public static class StepInertialConverter
    {
        public static Inertial ToInertial(StepMassProperties properties)
        {
            return new Inertial
            {
                pos = properties.centreOfMass,
                quat = properties.inertiaTensorRotation,
                mass = (float)properties.massKilograms,
                diaginertia = properties.inertiaTensor,
                useFullinertia = false,
            };
        }
    }
}
#endif
