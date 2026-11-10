namespace Greyhound.MuJoCo
{
    // Create-menu path, default file name and order for each of the
    // package's ScriptableObjects; every [CreateAssetMenu] in the package
    // reads its values from here. Unity sorts entries by order and starts
    // a new separator group when consecutive orders differ by more than
    // 10, so orders 0-5 keep the six entries in one group, in this order.
    public static class MenuManager
    {
        public const string ROOT_FILE_NAME = "MuJoCoRoot";
        public const string ROOT = "Greyhound/MuJoCo Root";
        public const int ROOT_ORDER = 0;

        
        public const string JOINT_FILE_NAME = "MuJoCoJoint";
        public const string JOINT = "Greyhound/MuJoCo Joint";
        public const int JOINT_ORDER = 1;

        
        public const string ACTUATOR_FILE_NAME = "MuJoCoActuator";
        public const string ACTUATOR = "Greyhound/MuJoCo Actuator";
        public const int ACTUATOR_ORDER = 2;

        
        public const string SITE_FILE_NAME = "MuJoCoSite";
        public const string SITE = "Greyhound/MuJoCo Site";
        public const int SITE_ORDER = 3;


        public const string GEOM_FILE_NAME = "MuJoCoGeom";
        public const string GEOM = "Greyhound/MuJoCo Geom";
        public const int GEOM_ORDER = 4;


        public const string SENSOR_FILE_NAME = "MuJoCoSensor";
        public const string SENSOR = "Greyhound/MuJoCo Sensor";
        public const int SENSOR_ORDER = 5;
    }
}
