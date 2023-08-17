import { APIGatewayEvent, APIGatewayProxyResult, Context } from "aws-lambda";
import { S3Client, GetObjectCommand } from "@aws-sdk/client-s3";
import pica from "pica";
import * as jpegjs from "jpeg-js";

const pkgName = "kumori-lambda-ts-pica";
let invokeCounter = 0;

async function readFromBucket(bucket: string, key: string): Promise<Uint8Array> {
  const s3client = new S3Client({ region: process.env.AWS_REGION });

  console.log(`S3://${bucket}/${key} region:${s3client.config.region}`);

  const command = new GetObjectCommand({ Bucket: bucket, Key: key });
  const s3object = await s3client.send(command);
  const bytes = await s3object.Body?.transformToByteArray();
  if (bytes === undefined) {
    throw new Error(`S3://${bucket}/${key} Failed to read!`);
  }

  console.log(`S3://${bucket}/${key} Read ${bytes.length / 1024.0}KiB`);

  return bytes;
}

export const handler = async (event: APIGatewayEvent, _context: Context): Promise<APIGatewayProxyResult> => {
  invokeCounter++;
  console.log(`Request: ${JSON.stringify(event, null, 2)}`);

  const filename = event.pathParameters?.proxy ?? "";
  const width_str = event.queryStringParameters?.width;
  const height_str = event.queryStringParameters?.height;

  const bucket = process.env.KUMORI_BUCKET!;

  const s3readStart = Date.now();
  const originalData = await readFromBucket(bucket, filename);
  const s3readDur = Date.now() - s3readStart;

  const resizeStart = Date.now();
  let finalData: Buffer;
  if (width_str && height_str) {
    const [width, height] = [parseInt(width_str), parseInt(height_str)];

    // Decode JPEG
    const rawData = jpegjs.decode(originalData);

    // Resize the data
    const resizedDate = await pica().resizeBuffer({
      src: rawData.data,
      width: rawData.width,
      height: rawData.height,
      toWidth: width,
      toHeight: height,
    });

    // Encode JPEG
    finalData = jpegjs.encode({ data: resizedDate, width, height }, 90).data;
  } else {
    finalData = Buffer.from(originalData);
  }
  const resizeDur = Date.now() - resizeStart;

  const serverTiming = `s3read;dur=${s3readDur.toFixed(3)},resize;dur=${resizeDur.toFixed(3)}`;

  return {
    statusCode: 200,
    headers: {
      "content-type": "image/jpeg",
      "server": pkgName,
      "server-timing": serverTiming,
      "x-invoke-count": `${invokeCounter}`,
    },
    body: finalData.toString("base64"),
    isBase64Encoded: true,
  };
};
