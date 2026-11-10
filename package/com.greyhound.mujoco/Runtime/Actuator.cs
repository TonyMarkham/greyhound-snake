using System;

using UnityEngine;

namespace Greyhound.MuJoCo
{
    // The serialized mirror of the MJCF <position> and <velocity>
    // actuator elements: every attribute of `element position` and
    // `element velocity` in mjcf-3.15.0.schema (with their groups
    // actuator_base, actuator_dynamics and transmission), in schema
    // order, names verbatim where C# allows. `type` selects the element.
    // `name` is the MJCF name; when empty the exporter uses the owning asset's name.
    //
    // <velocity> has every attribute here except inheritrange, kp,
    // dampratio and timeconst, which are <position>-only; MuJoCo rejects
    // them on <velocity> ("Schema violation: unrecognized attribute").
    //
    // Transmission targets (joint, jointinparent, site, refsite,
    // slidersite, cranksite) are object references on the owning
    // ScriptableObjects.Actuator asset, not fields here. tendon has no
    // counterpart: the package has no tendon type.
    //
    // kp and kv: the schema gives no default; the XML reference gives
    // <position> kp = 1, kv = 0, and <velocity> kv = 1. The fields carry
    // the <position> values, so a Velocity asset must author its kv.
    //
    // Units: ctrlrange is in the controlled quantity's units (joint
    // units for Position, joint velocity units for Velocity); forcerange
    // in N or N·m. Fixed two-value ranges are split into Lo/Hi fields;
    // variable-length lists (double[1..n]) are arrays.
    [Serializable]
    public sealed class Actuator
    {
        // MJCF name. Empty: the exporter writes the owning asset's name.
        // [Delayed]: applied on Enter or focus loss, because a change
        // renames the asset (ElementEditor).
        [Delayed]
        public string name;

        // The element selector (not a schema attribute).
        public ActuatorType type;

        // Defaults class; empty = none. `class` is a C# keyword.
        public string @class;

        public int group;

        // History buffer size for delay; 0 = none.
        public int nsample;

        public Interp interp;

        public float delay;

        // ctrlrange (double[2]).
        public float ctrlLo;

        public float ctrlHi;

        // Custom parameters; MJCF expects size/nuser_actuator entries.
        public float[] user = Array.Empty<float>();

        public FalseTrueAuto ctrllimited = FalseTrueAuto.Auto;

        public FalseTrueAuto forcelimited = FalseTrueAuto.Auto;

        // <position> only. Exclusive with an authored ctrlrange.
        public float inheritrange;

        // forcerange (double[2]).
        public float forceLo;

        public float forceHi;

        // lengthrange (double[2]).
        public float lengthrangeLo;

        public float lengthrangeHi;

        // 1 to 6 values; only the first matters for a slide/hinge joint.
        public float[] gear = { 1f, 0f, 0f, 0f, 0f, 0f };

        // Damper polynomial, 1 to 3 coefficients.
        public float[] damping = { 0f };

        public float armature;

        // Slider-crank transmissions only.
        public float cranklength;

        // <position> only.
        public float kp = 1f;

        // <position>: actuator damping. <velocity>: the servo gain.
        public float kv;

        // <position> only. Exclusive with kv.
        public float dampratio;

        // <position> only.
        public float timeconst;
    }
}
