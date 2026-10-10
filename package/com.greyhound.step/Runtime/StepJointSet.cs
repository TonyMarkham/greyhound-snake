using System.Collections.Generic;

using UnityEngine;

namespace Greyhound.Step
{
    // Container asset for all joint annotation of one STEP file. Lives in
    // "<step-stem>.joints/" beside the .stp it annotates. The step asset
    // path is the durable link: a serialized reference to the imported
    // root GameObject dies on every reimport, but the path is stable.
    // Joints and actuators are direct object references to their own
    // .asset files in the same folder; no GUIDs.
    [CreateAssetMenu(fileName = "StepJointSet", menuName = "Greyhound/Step Joint Set")]
    public sealed class StepJointSet : ScriptableObject
    {
        [StepAssetPath]
        public string stepAssetPath;

        // Mobility of the assembly root against the world: free gives the
        // exporter a free joint, welded writes no joint at all.
        public StepRootMobilityMode rootMobility;

        public List<StepJoint> joints = new List<StepJoint>();

        public List<StepActuator> actuators = new List<StepActuator>();
    }
}
