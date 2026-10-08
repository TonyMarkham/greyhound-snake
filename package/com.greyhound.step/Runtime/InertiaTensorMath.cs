using System;

using UnityEngine;

namespace Greyhound.Step
{
    // Dense symmetric 3x3 inertia math for the assembly roll-up: densify a
    // principal tensor into a frame, rotate it between frames, apply the
    // parallel-axis shift, and recover principal form with a Jacobi
    // eigendecomposition. Doubles throughout; the components convert at
    // their boundaries. Quaternion conventions follow Unity: q maps local
    // vectors into the parent frame (v_parent = q * v_local).
    internal static class InertiaTensorMath
    {
        // Writes the rotation matrix of q into r (row-major 3x3).
        private static void RotationMatrix(Quaternion q, double[,] r)
        {
            double x = q.x;
            double y = q.y;
            double z = q.z;
            double w = q.w;
            r[0, 0] = 1.0 - 2.0 * (y * y + z * z);
            r[0, 1] = 2.0 * (x * y - w * z);
            r[0, 2] = 2.0 * (x * z + w * y);
            r[1, 0] = 2.0 * (x * y + w * z);
            r[1, 1] = 1.0 - 2.0 * (x * x + z * z);
            r[1, 2] = 2.0 * (y * z - w * x);
            r[2, 0] = 2.0 * (x * z - w * y);
            r[2, 1] = 2.0 * (y * z + w * x);
            r[2, 2] = 1.0 - 2.0 * (x * x + y * y);
        }

        // tensor = R diag(diagonal) R^T: the dense tensor of the principal
        // form (diagonal moments, rotation into the principal frame).
        public static void Densify(Quaternion rotation, Vector3 diagonal, double[,] tensor)
        {
            var r = new double[3, 3];
            RotationMatrix(rotation, r);
            double[] t = { diagonal.x, diagonal.y, diagonal.z };
            for (int i = 0; i < 3; ++i)
            {
                for (int j = 0; j < 3; ++j)
                {
                    double sum = 0.0;
                    for (int k = 0; k < 3; ++k)
                    {
                        sum += r[i, k] * t[k] * r[j, k];
                    }

                    tensor[i, j] = sum;
                }
            }
        }

        // result = R tensor R^T: re-express the tensor in a frame rotated
        // by q relative to the tensor's frame.
        public static void Rotate(double[,] tensor, Quaternion rotation, double[,] result)
        {
            var r = new double[3, 3];
            RotationMatrix(rotation, r);
            for (int i = 0; i < 3; ++i)
            {
                for (int j = 0; j < 3; ++j)
                {
                    double sum = 0.0;
                    for (int k = 0; k < 3; ++k)
                    {
                        for (int l = 0; l < 3; ++l)
                        {
                            sum += r[i, k] * tensor[k, l] * r[j, l];
                        }
                    }

                    result[i, j] = sum;
                }
            }
        }

        // result = tensor + mass * (|d|^2 E - d d^T): shift from the
        // tensor's reference point to a point displaced by d.
        public static void ShiftAbout(
            double[,] tensor, double mass, double dx, double dy, double dz, double[,] result)
        {
            double[] d = { dx, dy, dz };
            double d2 = dx * dx + dy * dy + dz * dz;
            for (int i = 0; i < 3; ++i)
            {
                for (int j = 0; j < 3; ++j)
                {
                    double delta = i == j ? d2 : 0.0;
                    result[i, j] = tensor[i, j] + mass * (delta - d[i] * d[j]);
                }
            }
        }

        // result = tensor - mass * (|c|^2 E - c c^T): shift a total computed
        // about a fixed reference point to the centre of mass at c.
        public static void ShiftToCentre(
            double[,] tensor, double mass, double cx, double cy, double cz, double[,] result)
        {
            double[] c = { cx, cy, cz };
            double c2 = cx * cx + cy * cy + cz * cz;
            for (int i = 0; i < 3; ++i)
            {
                for (int j = 0; j < 3; ++j)
                {
                    double delta = i == j ? c2 : 0.0;
                    result[i, j] = tensor[i, j] - mass * (delta - c[i] * c[j]);
                }
            }
        }

        // Jacobi eigendecomposition of a symmetric 3x3: eigenvalues come
        // back in the order of the converged diagonal, eigenvectors as
        // columns of `eigenvectors`, with the determinant forced to +1 so
        // the columns are a proper rotation.
        public static void Eigendecompose(
            double[,] symmetric, double[] eigenvalues, double[,] eigenvectors)
        {
            var a = (double[,])symmetric.Clone();
            var v = new double[3, 3];
            for (int i = 0; i < 3; ++i)
            {
                v[i, i] = 1.0;
            }

            for (int sweep = 0; sweep < 64; ++sweep)
            {
                double off = (a[0, 1] * a[0, 1]) + (a[0, 2] * a[0, 2]) + (a[1, 2] * a[1, 2]);
                double scale = Math.Abs(a[0, 0]) + Math.Abs(a[1, 1]) + Math.Abs(a[2, 2]);
                if (off <= 1e-24 * scale * scale)
                {
                    break;
                }

                for (int pair = 0; pair < 3; ++pair)
                {
                    int p = pair == 2 ? 1 : 0;
                    int q = pair == 0 ? 1 : 2;
                    double apq = a[p, q];
                    double threshold = 1e-14 * (Math.Abs(a[p, p]) + Math.Abs(a[q, q]));
                    if (Math.Abs(apq) <= threshold + 1e-300)
                    {
                        continue;
                    }

                    double app = a[p, p];
                    double aqq = a[q, q];
                    double theta = (aqq - app) / (2.0 * apq);
                    double t = theta >= 0.0
                        ? 1.0 / (theta + Math.Sqrt((theta * theta) + 1.0))
                        : -1.0 / (-theta + Math.Sqrt((theta * theta) + 1.0));
                    double c = 1.0 / Math.Sqrt((t * t) + 1.0);
                    double s = t * c;

                    for (int k = 0; k < 3; ++k)
                    {
                        if (k == p || k == q)
                        {
                            continue;
                        }

                        double akp = a[k, p];
                        double akq = a[k, q];
                        double nkp = (c * akp) - (s * akq);
                        double nkq = (s * akp) + (c * akq);
                        a[k, p] = nkp;
                        a[p, k] = nkp;
                        a[k, q] = nkq;
                        a[q, k] = nkq;
                    }

                    a[p, p] = app - (t * apq);
                    a[q, q] = aqq + (t * apq);
                    a[p, q] = 0.0;
                    a[q, p] = 0.0;

                    for (int k = 0; k < 3; ++k)
                    {
                        double vkp = v[k, p];
                        double vkq = v[k, q];
                        v[k, p] = (c * vkp) - (s * vkq);
                        v[k, q] = (s * vkp) + (c * vkq);
                    }
                }
            }

            for (int i = 0; i < 3; ++i)
            {
                eigenvalues[i] = a[i, i];
                for (int j = 0; j < 3; ++j)
                {
                    eigenvectors[i, j] = v[i, j];
                }
            }

            double det =
                (eigenvectors[0, 0] * ((eigenvectors[1, 1] * eigenvectors[2, 2]) - (eigenvectors[1, 2] * eigenvectors[2, 1])))
                - (eigenvectors[0, 1] * ((eigenvectors[1, 0] * eigenvectors[2, 2]) - (eigenvectors[1, 2] * eigenvectors[2, 0])))
                + (eigenvectors[0, 2] * ((eigenvectors[1, 0] * eigenvectors[2, 1]) - (eigenvectors[1, 1] * eigenvectors[2, 0])));
            if (det < 0.0)
            {
                for (int k = 0; k < 3; ++k)
                {
                    eigenvectors[k, 2] = -eigenvectors[k, 2];
                }
            }
        }

        // The quaternion whose rotation matrix has the given orthonormal
        // columns (Shepperd's method; columns are the principal axes).
        public static Quaternion Rotation(double[,] columns)
        {
            double m00 = columns[0, 0];
            double m01 = columns[0, 1];
            double m02 = columns[0, 2];
            double m10 = columns[1, 0];
            double m11 = columns[1, 1];
            double m12 = columns[1, 2];
            double m20 = columns[2, 0];
            double m21 = columns[2, 1];
            double m22 = columns[2, 2];
            double trace = m00 + m11 + m22;
            double x;
            double y;
            double z;
            double w;
            if (trace > 0.0)
            {
                double s = Math.Sqrt(trace + 1.0) * 2.0;
                w = 0.25 * s;
                x = (m21 - m12) / s;
                y = (m02 - m20) / s;
                z = (m10 - m01) / s;
            }
            else if (m00 > m11 && m00 > m22)
            {
                double s = Math.Sqrt(1.0 + m00 - m11 - m22) * 2.0;
                w = (m21 - m12) / s;
                x = 0.25 * s;
                y = (m01 + m10) / s;
                z = (m02 + m20) / s;
            }
            else if (m11 > m22)
            {
                double s = Math.Sqrt(1.0 + m11 - m00 - m22) * 2.0;
                w = (m02 - m20) / s;
                x = (m01 + m10) / s;
                y = 0.25 * s;
                z = (m12 + m21) / s;
            }
            else
            {
                double s = Math.Sqrt(1.0 + m22 - m00 - m11) * 2.0;
                w = (m10 - m01) / s;
                x = (m02 + m20) / s;
                y = (m12 + m21) / s;
                z = 0.25 * s;
            }

            return new Quaternion((float)x, (float)y, (float)z, (float)w);
        }
    }
}
