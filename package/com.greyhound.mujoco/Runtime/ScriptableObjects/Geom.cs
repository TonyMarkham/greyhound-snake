using UnityEngine;

namespace Greyhound.MuJoCo.ScriptableObjects
{
    // One geom on a part: the counterpart of StepGeom. It belongs to the
    // Root whose `geoms` list holds it. `body` is the GameObject the
    // geom belongs to. The MJCF payload lives in `geom`
    // (Greyhound.MuJoCo.Geom).
    //
    // The GameObject `body` resolves to may have a MeshFilter. With the
    // step package its mesh exports as a default geom (the importer's
    // StepGeomProperties, converted at export). overrideDefault decides
    // how this asset relates to it:
    //   false (default): both export: the MeshFilter mesh as the default
    //         geom, and this geom.
    //   true: the MeshFilter mesh is not exported at all (no geom, no
    //         STL, no <asset> entry). If any geom asset on the GameObject
    //         overrides, the MeshFilter mesh is dropped. The override
    //         also replaces the GameObject's mass and inertia: its
    //         StepMassProperties <inertial> is not written, and MuJoCo
    //         derives them from the written geoms (geom mass, else
    //         density x volume; density defaults to 1000).
    // The payload type defaults to Null, which writes no <geom>. A Null
    // geom always overrides, whatever overrideDefault says (the Geom
    // inspector also checks it): without a shape there are no reliable
    // inertia properties, so the GameObject's MeshFilter mesh and its
    // default <inertial> are both dropped, and nothing is added in their
    // place: no geometry, no mass. A Null geom never carries inertia.
    //
    // Inside this namespace a bare `Geom` is this asset class, so the
    // payload type is written as MuJoCo.Geom.
    [CreateAssetMenu(fileName = MenuManager.GEOM_FILE_NAME, menuName = MenuManager.GEOM, order = MenuManager.GEOM_ORDER)]
    [Icon("Packages/com.greyhound.mujoco/Editor/Icons/Geom.png")]
    public sealed class Geom : ScriptableObject
    {
        public SiblingPath body;
        public bool overrideDefault;
        public MuJoCo.Geom geom = new MuJoCo.Geom();
    }
}
