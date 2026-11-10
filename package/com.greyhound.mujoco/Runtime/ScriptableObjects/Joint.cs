using UnityEngine;

namespace Greyhound.MuJoCo.ScriptableObjects
{
    // One joint on a part: the counterpart of StepJoint. It belongs to
    // the Root whose `joints` list holds it (as a sub-asset of that Root
    // when made with the Root's Add button). `body` is the part the joint
    // is defined in. The MJCF payload lives in `joint`
    // (Greyhound.MuJoCo.Joint). Units are MJCF-native: pos and slide
    // range in meters, hinge range in radians.
    //
    // Inside this namespace a bare `Joint` is this asset class, so the
    // payload type is written as MuJoCo.Joint (Greyhound.MuJoCo.Joint).
    [CreateAssetMenu(fileName = MenuManager.JOINT_FILE_NAME, menuName = MenuManager.JOINT, order = MenuManager.JOINT_ORDER)]
    [Icon("Packages/com.greyhound.mujoco/Editor/Icons/Joint.png")]
    public sealed class Joint : ScriptableObject
    {
        public SiblingPath body;
        public MuJoCo.Joint joint = new MuJoCo.Joint();
    }
}
