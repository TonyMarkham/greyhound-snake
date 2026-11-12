using Greyhound.Utilities;

using UnityEngine;

namespace Greyhound.MuJoCo.ScriptableObjects
{
    // One MJCF <framepos> sensor. Every
    // step MuJoCo writes the measured object's 3D position into
    // mjData.sensordata, in meters: in world coordinates, or, with a
    // reference, in the reference's frame. It belongs to the Root whose
    // `sensors` list holds it. The shared sensor attributes (group
    // sensor_base), including the MJCF name, live in `sensor`
    // (Greyhound.MuJoCo.Sensor).
    //
    // The schema's frame_object and frame_reference groups, field for
    // field:
    //   objtype (required): which kind of object is measured.
    //   objname (required): that object. MJCF names it by string; here it
    //     is a reference to the element, one field per kind, and only the
    //     field matching objtype is used:
    //       Site        -> objnameSite (a Site asset)
    //       Geom        -> objnameGeom (a Geom asset, not of type Null)
    //       Body, XBody -> objnameBody (the GameObject exported as that
    //                      <body>; xbody is its inertial frame)
    //       Camera      -> no camera type exists; the export is refused.
    //   reftype, refname (optional, together): the frame of reference, by
    //     the same rule with the refname* fields. When the field matching
    //     reftype is empty, neither is written: world coordinates.
    //
    // Inside this namespace a bare `Sensor` is this asset class, so the
    // payload type is written as MuJoCo.Sensor.
    [CreateAssetMenu(fileName = MenuManager.SENSOR_FILE_NAME, menuName = MenuManager.SENSOR, order = MenuManager.SENSOR_ORDER)]
    [Icon("Packages/com.greyhound.mujoco/Editor/Icons/Sensor.png")]
    public sealed class Sensor : ScriptableObject
    {
        public FrameObject objtype = FrameObject.Site;
        public Site objnameSite;
        public Geom objnameGeom;
        public SiblingPath objnameBody;

        public FrameObject reftype = FrameObject.Site;
        public Site refnameSite;
        public Geom refnameGeom;
        public SiblingPath refnameBody;

        public MuJoCo.Sensor sensor = new MuJoCo.Sensor();
    }
}
