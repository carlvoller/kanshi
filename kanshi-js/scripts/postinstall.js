
if (process.env.KANSHI_SKIP_POSTINSTALL) {
  process.exit(0);
}

const { platform, arch } = process;

const PLATFORMS = {
  darwin: {
    arm64: "darwin-arm64",
    x64: "darwin-x64",
  },
  linux: {
    arm64: "linux-arm64-gnu",
    x64: "linux-x64-gnu",
  },
  // win32: {
  //   x64: "win32-x64-msvc",
  // },
};

const platformSuffix = PLATFORMS?.[platform]?.[arch];

if (!platform) {

  const supportedPlatforms = Object.keys(PLATFORMS).map((x) => `${x} - ${Object.keys(PLATFORMS[x])}`)

  console.error(
    `No native library is available for your platform/cpu (${platform}/${arch}). Kanshi only supports the following platforms: ${supportedPlatforms}`,
  );
  process.exit(2);
}

const libName = `@kanshi-js/${platformSuffix}/index.node`;
let path;
try {
  path = require.resolve(libName);
} catch (err) {
  console.error(
    `Failed to install native libs for Kanshi. ${err}`,
    "\n",
    "",
  );
  process.exit(3);
}