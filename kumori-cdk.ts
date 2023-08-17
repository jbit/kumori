import { Construct } from "constructs";
import { App, CfnOutput, Duration, RemovalPolicy, Stack, StackProps, Tags } from "aws-cdk-lib";
import { IFunction, Runtime } from "aws-cdk-lib/aws-lambda";
import { NodejsFunction, NodejsFunctionProps } from "aws-cdk-lib/aws-lambda-nodejs";
import { RustFunction, RustFunctionProps } from "cargo-lambda-cdk";
import { Bucket, BlockPublicAccess } from "aws-cdk-lib/aws-s3";
import { HttpLambdaIntegration } from "@aws-cdk/aws-apigatewayv2-integrations-alpha";
import { HttpApi, HttpMethod } from "@aws-cdk/aws-apigatewayv2-alpha";

const DEFAULT_LAMBDA_PROPS_NODEJS: NodejsFunctionProps = {
  runtime: Runtime.NODEJS_18_X,
  bundling: { minify: true },
  timeout: Duration.seconds(30),
  memorySize: 512,
};

const DEFAULT_LAMBDA_PROPS_RUST: RustFunctionProps = {
  timeout: Duration.seconds(30),
  memorySize: 512,
};

class KumoriStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);
    const outputs: CfnOutput[] = [];

    const bucket = new Bucket(this, "Bucket", {
      blockPublicAccess: BlockPublicAccess.BLOCK_ALL,
      removalPolicy: RemovalPolicy.DESTROY,
    });
    outputs.push(new CfnOutput(this, "BucketName", { value: bucket.bucketName }));

    const lambdas: { [index: string]: IFunction } = {};
    lambdas["ts-sharp"] = new NodejsFunction(this, "LambdaTsSharp", {
      ...DEFAULT_LAMBDA_PROPS_NODEJS,
      handler: "handler",
      entry: "lambda-ts-sharp/index.ts",
      environment: { KUMORI_BUCKET: bucket.bucketName },
      bundling: {
        // Hack to support cross building the sharp C library from macos
        externalModules: ["sharp"],
        nodeModules: ["sharp"],
        commandHooks: {
          beforeBundling(_inputDir: string, _outputDir: string): string[] {
            return [];
          },
          beforeInstall(_inputDir: string, _outputDir: string): string[] {
            return [];
          },
          afterBundling(_inputDir: string, outputDir: string): string[] {
            return [`cd ${outputDir}`, "rm -rf node_modules/sharp && npm install --arch=x64 --platform=linux sharp"];
          },
        },
      },
    });

    lambdas["ts-pica"] = new NodejsFunction(this, "LambdaTsPica", {
      ...DEFAULT_LAMBDA_PROPS_NODEJS,
      handler: "handler",
      entry: "lambda-ts-pica/index.ts",
      environment: { KUMORI_BUCKET: bucket.bucketName },
      bundling: {
        externalModules: ["pica", "jpeg-js"],
        nodeModules: ["pica", "jpeg-js"],
      },
    });

    lambdas["rs-fir"] = new RustFunction(this, "LambdaRsFir", {
      ...DEFAULT_LAMBDA_PROPS_RUST,
      binaryName: "kumori-lambda-rs-fir",
      manifestPath: "lambda-rs-fir/Cargo.toml",
      environment: { KUMORI_BUCKET: bucket.bucketName },
    });

    // Link lambdas to API gateway, and give them permissions
    const httpApi = new HttpApi(this, `${this.stackName}Api`, {});
    for (const [variant, lambda] of Object.entries(lambdas)) {
      const integration = new HttpLambdaIntegration(`${lambda.node.id}Integration`, lambda);
      httpApi.addRoutes({ path: `/${variant}/{proxy+}`, integration: integration, methods: [HttpMethod.GET] });
      bucket.grantRead(lambda);
      outputs.push(
        new CfnOutput(this, `EndpointUrl${lambda.node.id}`, { value: `${httpApi.apiEndpoint}/${variant}/` })
      );
    }
  }
}

const app = new App();
const stack = new KumoriStack(app, "Kumori", {});
Tags.of(app).add("app", stack.stackName);
