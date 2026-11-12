using UnityEngine;

namespace Greyhound.MuJoCo.ScriptableObjects
{
    // One actuator. It belongs to the
    // Root whose `actuators` list holds it. The MJCF payload lives in
    // `actuator` (Greyhound.MuJoCo.Actuator).
    //
    // Transmission: the schema's transmission attributes are references
    // to other elements, so they are object references here. `target` is
    // the `joint` attribute; jointinparent, site, refsite, slidersite and
    // cranksite are the others. MJCF requires exactly one transmission
    // per actuator (refsite only accompanies site; slidersite only
    // accompanies cranksite). tendon has no counterpart: the package has
    // no tendon type.
    //
    // Inside this namespace a bare `Actuator` is this asset class, so the
    // payload type is written as MuJoCo.Actuator.
    [CreateAssetMenu(fileName = MenuManager.ACTUATOR_FILE_NAME, menuName = MenuManager.ACTUATOR, order = MenuManager.ACTUATOR_ORDER)]
    [Icon("Packages/com.greyhound.mujoco/Editor/Icons/Actuator.png")]
    public sealed class Actuator : ScriptableObject
    {
        public Joint target;
        public Joint jointinparent;
        public Site site;
        public Site refsite;
        public Site slidersite;
        public Site cranksite;
        public MuJoCo.Actuator actuator = new MuJoCo.Actuator();
    }
}
