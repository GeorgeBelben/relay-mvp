const UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

// Binary (1024-based) units to match what statvfs-derived numbers and every OS disk-usage UI
// actually mean by "GB" here.
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";

  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < UNITS.length - 1) {
    value /= 1024;
    unitIndex++;
  }

  return `${unitIndex === 0 ? value : value.toFixed(1)} ${UNITS[unitIndex]}`;
}
