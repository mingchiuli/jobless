import { isAbsolute, relative, resolve, sep } from "node:path";

export function validatePlatformId(platformId: string): void {
  if (
    platformId.length === 0 ||
    platformId.length > 64 ||
    !/^[a-z0-9_-]+$/.test(platformId)
  ) {
    throw new Error(`invalid platform id: ${platformId}`);
  }
}

export function validateProfileId(profileId: string): void {
  if (
    !/^[A-Za-z0-9._-]+$/.test(profileId) ||
    profileId === "." ||
    profileId === ".."
  ) {
    throw new Error(`invalid browser profile id: ${profileId}`);
  }
}

export function profileDirectory(profileRoot: string, profileId: string): string {
  validateProfileId(profileId);
  const root = resolve(profileRoot);
  const profile = resolve(root, profileId);
  const child = relative(root, profile);
  if (child === "" || child.startsWith(`..${sep}`) || child === ".." || isAbsolute(child)) {
    throw new Error(`browser profile escapes profile root: ${profileId}`);
  }
  return profile;
}

export function validateUrl(url: string): void {
  if (url === "about:blank") {
    return;
  }

  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`invalid browser URL: ${url}`);
  }

  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error(`unsupported browser URL scheme: ${parsed.protocol}`);
  }
}
