const DEFAULT_OUTPUT_FILE = "unicode_flash_mob.mp4";

export function applyOutputDirectory(outputPath: unknown, outputDir: string): string {
  const fileName =
    String(outputPath || DEFAULT_OUTPUT_FILE).split(/[\\/]/).pop() ||
    DEFAULT_OUTPUT_FILE;
  if (!outputDir) return String(outputPath || DEFAULT_OUTPUT_FILE);
  return `${outputDir.replace(/[\\/]$/, "")}/${fileName}`;
}

export function getSidecarConfigPath(outputPath: unknown): string {
  const path = String(outputPath || DEFAULT_OUTPUT_FILE);
  return /\.[^./\\]+$/.test(path)
    ? path.replace(/\.[^./\\]+$/, ".json")
    : `${path}.json`;
}
