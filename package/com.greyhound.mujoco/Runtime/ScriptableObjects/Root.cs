using System.Collections.Generic;

using Greyhound.Utilities;

using UnityEngine;

namespace Greyhound.MuJoCo.ScriptableObjects
{
    // Container asset for the MuJoCo annotation of one model.
    // Joints, actuators, sites, geoms and
    // sensors are direct object references to their own .asset files;
    // each of them points back here through its `root`.
    [CreateAssetMenu(fileName = MenuManager.ROOT_FILE_NAME, menuName = MenuManager.ROOT, order = MenuManager.ROOT_ORDER)]
    public sealed class Root : ScriptableObject
    {
        // The model this Root annotates and the exporter writes: a prefab
        // or model asset (a ScriptableObject cannot reference scene
        // objects). Each element's `body` (SiblingPath) also names a root
        // asset; for an element to export it must be this one.
        public GameObject root;
        // Mobility of the assembly root against the world: free gives the
        // exporter a free joint, welded writes no joint at all.
        public RootMobilityMode rootMobility;
        // The MJCF <option>: every attribute of `element option` in
        // mjcf-3.15.0.schema, in schema order, with the schema's defaults
        // except integrator (ImplicitFast, decided for the velocity
        // actuator; schema default Euler). timestep is the physics step
        // in seconds and must be positive. Vectors are in Unity axes and
        // exported as (x, z, y): gravity (0, -9.81, 0) is MJCF
        // "0 0 -9.81", magnetic (0, 0, -0.5) is MJCF "0 -0.5 0".
        public float timestepSeconds = 0.002f;
        public float impratio = 1f;
        public float tolerance = 1e-8f;
        public float ls_tolerance = 0.01f;
        public float noslip_tolerance = 1e-6f;
        public float ccd_tolerance = 1e-6f;
        public float sleep_tolerance = 0.001f;
        public Vector3 gravity = new Vector3(0f, -9.81f, 0f);
        public Vector3 wind = Vector3.zero;
        public Vector3 magnetic = new Vector3(0f, 0f, -0.5f);
        public float density;
        public float viscosity;
        public float o_margin;
        public float[] o_solref = { 0.02f, 1f };
        public float[] o_solimp = { 0.9f, 0.95f, 0.001f, 0.5f, 2f };
        public float[] o_friction = { 1f, 1f, 0.005f, 0.0001f, 0.0001f };
        public Integrator integrator = Integrator.ImplicitFast;
        public Cone cone = Cone.Pyramidal;
        public Jacobian jacobian = Jacobian.Auto;
        public Solver solver = Solver.Newton;
        public int iterations = 100;
        public int ls_iterations = 50;
        public int noslip_iterations;
        public int ccd_iterations = 35;
        public int sdf_iterations = 10;
        public int sdf_initpoints = 40;
        // Actuator groups to disable (each listed group's bit is set).
        public int[] actuatorgroupdisable = System.Array.Empty<int>();
        // The <option><flag> child element.
        public Flag flag = new Flag();
        public List<Joint> joints = new List<Joint>();
        public List<Actuator> actuators = new List<Actuator>();
        public List<Site> sites = new List<Site>();
        public List<Geom> geoms = new List<Geom>();
        public List<Sensor> sensors = new List<Sensor>();
    }
}
