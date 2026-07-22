const videoPath = new URL(
  "../out/wrec-docs-release-lakebed.mp4",
  import.meta.url,
);
const outputPath = new URL("../lakebed/client/video-data.ts", import.meta.url);
const video = Bun.file(videoPath);

if (!(await video.exists())) {
  throw new Error(`Missing Lakebed encode: ${videoPath.pathname}`);
}

const data = Buffer.from(await video.arrayBuffer()).toString("base64");
await Bun.write(
  outputPath,
  `export const VIDEO_DATA_URL = "data:video/mp4;base64,${data}";\n`,
);

console.log(`Embedded ${video.size} MP4 bytes into ${outputPath.pathname}`);
