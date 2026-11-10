using System;

using UnityEngine;

namespace Greyhound.MuJoCo
{
    // The serialized mirror of the attributes every MJCF sensor shares:
    // `group sensor_base` in mjcf-3.15.0.schema, in schema order. `name` is
    // the MJCF name; when empty the exporter uses the owning asset's name. What the sensor
    // measures (objtype/objname, reftype/refname for <framepos>) are
    // object references on the owning ScriptableObjects.Sensor.
    [Serializable]
    public sealed class Sensor
    {
        // MJCF name. Empty: the exporter writes the owning asset's name.
        // [Delayed]: applied on Enter or focus loss, because a change
        // renames the asset (ElementEditor).
        [Delayed]
        public string name;

        // History buffer size for delay; 0 = none.
        public int nsample;

        public Interp interp;

        public float delay;

        // 1 or 2 values; empty = unset.
        public float[] interval = Array.Empty<float>();

        public float cutoff;

        public float noise;

        // Custom parameters; MJCF expects size/nuser_sensor entries.
        public float[] user = Array.Empty<float>();
    }
}
