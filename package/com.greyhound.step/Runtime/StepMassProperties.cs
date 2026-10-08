using UnityEngine;

namespace Greyhound.Step
{
    // Mass properties of the part this node renders, computed from the
    // exact BRep on the native side. The density is the STEP file's
    // material density when the file carries one, otherwise the import
    // parameter. Values are Unity-ready: mass in kg, centre of mass in
    // body-local meters, inertia tensor as principal moments about the
    // centre of mass with the rotation into the principal frame.
    public sealed class StepMassProperties : MonoBehaviour
    {
        public double massKilograms;
        public Vector3 centreOfMass;
        public Vector3 inertiaTensor;
        public Quaternion inertiaTensorRotation;
        public Vector3 gyrationRadii;
        public double volumeMm3;
        public double densityGPerCm3;
        public bool usedFileDensity;

        public static double EffectiveDensity(in HostMeshProperties properties, double defaultDensity)
        {
            return properties.FileDensity > 0.0 ? properties.FileDensity : defaultDensity;
        }

        public static StepMassProperties Create(Transform node, in HostMeshProperties properties, double defaultDensity)
        {
            double density = EffectiveDensity(properties, defaultDensity);
            // mm3 -> cm3 (1e-3) times g/cm3 gives grams; grams -> kg divides
            // by 1000, so volume_mm3 * density * 1e-6 is kilograms.
            double massKg = (double)properties.VolumeMm3 * density * 1e-6;
            double r1 = properties.GyrationRadius1 * 0.001;
            double r2 = properties.GyrationRadius2 * 0.001;
            double r3 = properties.GyrationRadius3 * 0.001;
            var rotation = Matrix4x4.identity;
            rotation.SetColumn(0, new Vector4(properties.Axis1X, properties.Axis1Y, properties.Axis1Z, 0f));
            rotation.SetColumn(1, new Vector4(properties.Axis2X, properties.Axis2Y, properties.Axis2Z, 0f));
            rotation.SetColumn(2, new Vector4(properties.Axis3X, properties.Axis3Y, properties.Axis3Z, 0f));

            var component = node.gameObject.AddComponent<StepMassProperties>();
            component.massKilograms = massKg;
            component.centreOfMass = new Vector3(
                properties.CentreOfGravityX, properties.CentreOfGravityY, properties.CentreOfGravityZ);
            component.inertiaTensor = new Vector3(
                (float)(massKg * r1 * r1), (float)(massKg * r2 * r2), (float)(massKg * r3 * r3));
            component.inertiaTensorRotation = rotation.rotation;
            component.gyrationRadii = new Vector3(
                properties.GyrationRadius1, properties.GyrationRadius2, properties.GyrationRadius3);
            component.volumeMm3 = properties.VolumeMm3;
            component.densityGPerCm3 = density;
            component.usedFileDensity = properties.FileDensity > 0.0;
            return component;
        }

        public void ApplyTo(Rigidbody body)
        {
            body.mass = (float)massKilograms;
            body.centerOfMass = centreOfMass;
            body.inertiaTensor = inertiaTensor;
            body.inertiaTensorRotation = inertiaTensorRotation;
        }
    }
}
