using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Greyhound.Utilities
{
    public class ForceProjectRestart
    {
        [MenuItem("Utilities/Restart Unity Project")]
        private static void Restart()
        {
            EditorSceneManager.SaveOpenScenes();
            AssetDatabase.SaveAssets();
            EditorApplication.OpenProject(System.IO.Directory.GetParent(Application.dataPath)?.FullName);
        }
    }
}
