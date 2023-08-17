# Kumori

☁️🦀☁️🦀☁️

Simple CDK project for deploying Rust Lambdas and TypeScript lambdas and measuring simple performance metrics.

## Structure

| Location                               | Description                                                                |
| -------------------------------------- | -------------------------------------------------------------------------- |
| [kumori-cdk.ts](./kumori-cdk.ts)       | CDK stack for setting up lambdas and S3 buckets                            |
| [lambda-rs-fir/](./lambda-rs-fir/)     | Rust Lambda function using fast_image_resize (FIR) to scale images from S3 |
| [lambda-ts-pica/](./lambda-ts-pica/)   | TypeScript Lambda function using pica to scale images in pure JavaScript   |
| [lambda-ts-sharp/](./lambda-ts-sharp/) | TypeScript Lambda function using sharp to scale images using C (libvips)   |
| [measure/](./measure/)                 | Simple Rust tool to measure run time of Lambda functions executions        |

## Setup

The commands below assume you're on macOS with [Homebrew Package Manager](https://brew.sh/) installed. If you're using a different OS, please read the linked sites.

### Install AWS CLI

https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html

```
brew install awscli
```

### Configure your AWS CLI credentials

This depends a lot on how your AWS account is setup, please follow this guide.

https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-configure.html

### Install NodeJS

https://nodejs.org/en

```
brew install node
```

### Install Rust

https://rustup.rs/

```
brew install rustup-init
rustup-init
```

### Install Cargo Lambda

https://www.cargo-lambda.info/guide/installation.html

```
brew tap cargo-lambda/cargo-lambda
brew install cargo-lambda
```

### Download this repository

```
git clone git@github.com:jbit/kumori.git
```

### Setup NPM for kumori

```
npm install
```

## Deployment

### Check you are who you think you are

```
aws sts get-caller-identity --output table
```

> ```
> ----------------------------------------------------------------------
> |                         GetCallerIdentity                          |
> +----------+-------------------------------------------+-------------+
> | Account  | Arn                                       | UserId      |
> +----------+-------------------------------------------+-------------+
> | 13371337 | arn:aws:sts::13371337:assumed-role/popeye | WXYZ:popeye |
> +----------+-------------------------------------------+-------------+
> ```

```
aws iam list-account-aliases --output table
```

> ```
> ------------------------
> |  ListAccountAliases  |
> +----------------------+
> ||   AccountAliases   ||
> |+--------------------+|
> ||  rust-evaluation   ||
> |+--------------------+|
> ```

### Bootstrap CDK (only needed once per account region)

```
npx cdk bootstrap
```

> ```
> [...]
>  ✅  Environment aws://13371337/ap-southeast-2 bootstrapped.
> ```

### Deploy!

```
npx cdk deploy --outputs-file=out/cdk-outputs.json
```

### Push test data to S3 bucket

```
S3_URL="s3://$(jq -r .Kumori.BucketName out/cdk-outputs.json)"
aws s3 cp --recursive test-data/ "${S3_URL}"
```

## Basic automated performance measurement

Currently this uses timings from real-world HTTP request, this has a lot of variation based on network latencies. It's best to run this on an EC2 instance in the same region for best results.

Please sanity check the output against the CloudWatch logs. In the future this tool should use CloudWatch and X-Ray directly for metrics.

```
cargo run --bin kumori-measure --release out/cdk-outputs.json Crab-2592x1944.jpg 512 512
cargo run --bin kumori-measure --release out/cdk-outputs.json Ferris-1024x576.jpg 512 512
```

## Cleaning up

### Shutdown and remove AWS resources

```
npx cdk destroy
```

### Delete local build data

```
npm run clean
```

## Troubleshooting

### `Failed to bundle asset Kumori/LambdaRust/Code/Stage`

If you receive an error like the following, then you don't have `cargo-lambda` installed correctly.

```
Bundling asset Kumori/LambdaRust/Code/Stage...
Rust build cannot run locally. Switching to Docker bundling.
kumori/node_modules/aws-cdk-lib/core/lib/asset-staging.ts:468
      throw new Error(`Failed to bundle asset ${this.node.path}, bundle output is located at ${bundleErrorDir}: ${err}`);
            ^
Error: Failed to bundle asset Kumori/LambdaRust/Code/Stage, bundle output is located at kumori/cdk.out/bundling-temp-xyz-error: Error: spawnSync docker ENOENT
```
