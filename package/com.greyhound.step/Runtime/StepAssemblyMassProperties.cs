using UnityEngine;

namespace Greyhound.Step
{
    // Aggregated mass properties of an assembly node, computed on the Unity
    // side from the mass-properties components under this node: meshed
    // leaves contribute their StepMassProperties, nested assemblies
    // contribute their own StepAssemblyMassProperties (recalculated first,
    // and counted instead of their subtree). Values live in this
    // transform's local frame — mass in kg, centre of mass in meters,
    // inertia as principal moments about the centre of mass with the
    // rotation into the principal frame — so they stay correct wherever
    // the subtree is reparented. Recalculate is explicit: nothing refreshes
    // this component when the hierarchy changes.
    public sealed class StepAssemblyMassProperties : MonoBehaviour
    {
        public double massKilograms;
        public Vector3 centreOfMass;
        public Vector3 inertiaTensor;
        public Quaternion inertiaTensorRotation = Quaternion.identity;
        public int partCount;

        [ContextMenu("Recalculate")]
        public void Recalculate()
        {
            double mass = 0.0;
            double comX = 0.0;
            double comY = 0.0;
            double comZ = 0.0;
            var aboutOrigin = new double[3, 3];
            int parts = 0;
            foreach (Transform child in transform)
            {
                Collect(child, ref mass, ref comX, ref comY, ref comZ, aboutOrigin, ref parts);
            }

            if (parts == 0 || mass <= 0.0)
            {
                massKilograms = 0.0;
                centreOfMass = Vector3.zero;
                inertiaTensor = Vector3.zero;
                inertiaTensorRotation = Quaternion.identity;
                partCount = 0;
                return;
            }

            double centreX = comX / mass;
            double centreY = comY / mass;
            double centreZ = comZ / mass;
            var aboutCentre = new double[3, 3];
            InertiaTensorMath.ShiftToCentre(aboutOrigin, mass, centreX, centreY, centreZ, aboutCentre);

            var eigenvalues = new double[3];
            var eigenvectors = new double[3, 3];
            InertiaTensorMath.Eigendecompose(aboutCentre, eigenvalues, eigenvectors);

            massKilograms = mass;
            centreOfMass = new Vector3((float)centreX, (float)centreY, (float)centreZ);
            inertiaTensor = new Vector3((float)eigenvalues[0], (float)eigenvalues[1], (float)eigenvalues[2]);
            inertiaTensorRotation = InertiaTensorMath.Rotation(eigenvectors);
            partCount = parts;
        }

        // The dense tensor about this aggregate's centre of mass in this
        // node's local frame, from the stored principal form; ancestors
        // read this instead of re-walking the subtree.
        internal bool TryContribution(out double mass, out Vector3 centreOfMassLocal, out double[,] tensor)
        {
            mass = massKilograms;
            centreOfMassLocal = centreOfMass;
            tensor = new double[3, 3];
            if (mass <= 0.0)
            {
                return false;
            }

            InertiaTensorMath.Densify(inertiaTensorRotation, inertiaTensor, tensor);
            return true;
        }

        public void ApplyTo(Rigidbody body)
        {
            body.mass = (float)massKilograms;
            body.centerOfMass = centreOfMass;
            body.inertiaTensor = inertiaTensor;
            body.inertiaTensorRotation = inertiaTensorRotation;
        }

        private void Collect(
            Transform node,
            ref double mass,
            ref double comX,
            ref double comY,
            ref double comZ,
            double[,] aboutOrigin,
            ref int parts)
        {
            var assembly = node.GetComponent<StepAssemblyMassProperties>();
            if (assembly != null && assembly != this)
            {
                assembly.Recalculate();
                if (assembly.TryContribution(out double childMass, out Vector3 childCom, out double[,] tensor))
                {
                    Accumulate(node, childMass, childCom, tensor, assembly.partCount, ref mass, ref comX, ref comY, ref comZ, aboutOrigin, ref parts);
                }

                return;
            }

            var part = node.GetComponent<StepMassProperties>();
            if (part != null)
            {
                AccumulatePart(node, part, ref mass, ref comX, ref comY, ref comZ, aboutOrigin, ref parts);
                return;
            }

            foreach (Transform child in node)
            {
                Collect(child, ref mass, ref comX, ref comY, ref comZ, aboutOrigin, ref parts);
            }
        }

        private void AccumulatePart(
            Transform node,
            StepMassProperties part,
            ref double mass,
            ref double comX,
            ref double comY,
            ref double comZ,
            double[,] aboutOrigin,
            ref int parts)
        {
            if (part.massKilograms <= 0.0)
            {
                return;
            }

            var tensor = new double[3, 3];
            InertiaTensorMath.Densify(part.inertiaTensorRotation, part.inertiaTensor, tensor);
            Accumulate(node, part.massKilograms, part.centreOfMass, tensor, 1, ref mass, ref comX, ref comY, ref comZ, aboutOrigin, ref parts);
        }

        private void Accumulate(
            Transform node,
            double childMass,
            Vector3 childComLocal,
            double[,] childTensor,
            int partsToAdd,
            ref double mass,
            ref double comX,
            ref double comY,
            ref double comZ,
            double[,] aboutOrigin,
            ref int parts)
        {
            Vector3 world = node.TransformPoint(childComLocal);
            Vector3 local = transform.InverseTransformPoint(world);
            Quaternion relative = Quaternion.Inverse(transform.rotation) * node.rotation;
            var rotated = new double[3, 3];
            InertiaTensorMath.Rotate(childTensor, relative, rotated);
            InertiaTensorMath.ShiftAbout(rotated, childMass, local.x, local.y, local.z, rotated);

            mass += childMass;
            comX += childMass * local.x;
            comY += childMass * local.y;
            comZ += childMass * local.z;
            for (int i = 0; i < 3; ++i)
            {
                for (int j = 0; j < 3; ++j)
                {
                    aboutOrigin[i, j] += rotated[i, j];
                }
            }

            parts += partsToAdd;
        }
    }
}
