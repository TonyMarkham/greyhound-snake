using UnityEditor;

namespace Greyhound.Utilities
{
    public static class ForceDomainReload
    {
        [MenuItem("Utilities/Force Domain Reload")]
        private static void ForceReload() => EditorUtility.RequestScriptReload();
    }
}
