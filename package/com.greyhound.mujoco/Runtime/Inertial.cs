using System;

using UnityEngine;

namespace Greyhound.MuJoCo
{
    // The serialized mirror of the MJCF <inertial> element: every
    // attribute of `element inertial` in mjcf-3.15.0.schema, in schema
    // order. A body's mass properties in its own frame: kg, meters,
    // kg·m². When a body has no <inertial>, MuJoCo derives these from
    // its geoms (compiler inertiafromgeom="auto").
    //
    // pos and mass are required by the schema. Orientation is stored as
    // quat only, as for Site and Geom.
    //
    // Two ways to give the inertia, and the schema makes them exclusive
    // ("exclusive fullinertia quat ..."):
    //   - diaginertia: principal moments, in the principal frame that
    //     quat rotates to. Written when useFullinertia is false.
    //   - fullinertia: the full symmetric tensor in the body frame,
    //     [Ixx, Iyy, Izz, Ixy, Ixz, Iyz]. Written, with no quat, when
    //     useFullinertia is true.
    //
    // Unity axes, as everywhere in this package: pos is exported as
    // (x, z, y) and quat is conjugated into the MJCF frame. diaginertia
    // needs no swap (it follows quat). fullinertia does: Unity
    // [Ixx, Iyy, Izz, Ixy, Ixz, Iyz] exports as MJCF
    // [Ixx, Izz, Iyy, Ixz, Ixy, Iyz] (swapping y and z swaps those
    // indices; Iyz maps to itself because the tensor is symmetric).
    [Serializable]
    public sealed class Inertial
    {
        // Centre of mass in the body frame, meters.
        public Vector3 pos;

        public Quaternion quat = Quaternion.identity;

        // Kilograms; must be positive for a moving body.
        public float mass;

        // Principal moments, kg·m².
        public Vector3 diaginertia;

        public bool useFullinertia;

        // [Ixx, Iyy, Izz, Ixy, Ixz, Iyz], kg·m², in Unity axes.
        public float[] fullinertia = new float[6];
    }
}
