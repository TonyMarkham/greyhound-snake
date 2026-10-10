using UnityEngine;

namespace Greyhound.Step
{
    // The importer-derived physics entry for a meshed part: the
    // baseline geom the export uses when no StepGeom asset (authored
    // intent) exists for the part. Created by the importer beside
    // StepMassProperties and regenerated on every reimport — the same
    // derived-data lifecycle — so edits made on the component die with
    // the next reimport; persistent intent belongs in a StepGeom asset,
    // which the exporter prefers over this baseline.
    //
    // The baseline is visible but collisionless (contype 0 /
    // conaffinity 0): press-fit CAD parts interpenetrate volumetrically
    // and the compiler's default contact friction would otherwise turn
    // those pressed pairs into a brake. Deliberate collision is
    // authored on the part's StepGeom asset.
    public sealed class StepGeomProperties : MonoBehaviour
    {
        public MjGeom mj = new MjGeom();

        public static StepGeomProperties Create(Transform node)
        {
            var component = node.gameObject.AddComponent<StepGeomProperties>();
            component.mj.type = StepGeomType.Mesh;
            component.mj.contype = 0;
            component.mj.conaffinity = 0;
            return component;
        }
    }
}
