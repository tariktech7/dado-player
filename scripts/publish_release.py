"""Publish a GitHub release from the verified local release index."""
import argparse
import hashlib
import io
import json
import os
import re
import subprocess
import zipfile
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

from release_version import ROOT, github_release_tag, public_version, release_title

REPOSITORY = "tariktech7/dado-player"
API = f"https://api.github.com/repos/{REPOSITORY}/releases"


def git(*args, **kwargs):
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, check=True, text=True, **kwargs
    ).stdout.strip()


def access_token():
    if token := os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN"):
        return token
    credential = git(
        "credential", "fill", input="protocol=https\nhost=github.com\n\n",
        env={**os.environ, "GCM_INTERACTIVE": "never"},
    )
    for line in credential.splitlines():
        if line.startswith("password="):
            return line.removeprefix("password=")
    raise ValueError("GitHub credentials are missing")


def request(token, url, method="GET", *, metadata=None, payload=None):
    headers = {
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
        "User-Agent": "Dado-Player-Release",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if metadata is not None:
        headers["Content-Type"] = "application/json"
        payload = json.dumps(metadata).encode("utf-8")
    elif payload is not None:
        headers["Content-Type"] = "application/octet-stream"
    with urlopen(Request(url, payload, headers, method=method), timeout=120) as response:
        return json.load(response)


def digest(data):
    return f"sha256:{hashlib.sha256(data).hexdigest()}"


def deliveries(release, folder=ROOT / "Releases"):
    index = json.loads((folder / "AKTUELL.json").read_bytes())
    expected = {
        "Dado-Player.exe": f"windows-build-{release['windowsBuild']}",
        "Dado-Player-arm64-debug.apk": f"android-build-{release['androidBuild']}",
        "Dado-Player-unsigned.ipa": f"ios-build-{release['iosBuild']}",
    }
    version = public_version(release)
    expected[f"Dado-Player-Tizen-{version}-unsigned.wgt"] = ""
    expected[f"Dado-Player-Tizen5-{version}-unsigned.wgt"] = ""
    entries = {entry["file"]: entry for entry in index["packages"]}
    if not expected.keys() <= entries.keys() or len(entries) != len(index["packages"]):
        raise ValueError("The release index must contain each public package exactly once")
    result = {}
    for name, label in expected.items():
        item = entries[name]
        data = (folder / name).read_bytes()
        if (item["assetLabel"] or "") != label or digest(data) != f"sha256:{item['sha256']}" or len(data) != item["bytes"]:
            raise ValueError(f"The local release index does not match {name}")
        # A Samsung developer certificate can contain a TV's device ID (DUID).
        if name.endswith(".wgt"):
            with zipfile.ZipFile(io.BytesIO(data)) as widget:
                if any(re.fullmatch(r"(?:author-signature|signature\d+)\.xml", Path(entry).name, re.I)
                       for entry in widget.namelist()):
                    raise ValueError(f"A public TV package must be unsigned: {name}")
        result[name] = data, label
    index["packages"] = [entry for entry in index["packages"] if entry["file"] in expected]
    index["samsungUsb"].pop("preparationFile", None)
    result["AKTUELL.json"] = (json.dumps(index, indent=2, ensure_ascii=False) + "\n").encode("utf-8"), ""
    return result


def validate_asset(asset, data, label):
    if ((asset.get("label") or ""), asset.get("size"), asset.get("digest"), asset.get("state")) != (
        label, len(data), digest(data), "uploaded"
    ):
        raise ValueError(f"GitHub asset verification failed: {asset['name']}")


def publish(release, notes, folder=ROOT / "Releases"):
    tag = github_release_tag(release)
    prerelease = release["channel"] == "beta"
    title = release_title(release)
    files = deliveries(release, folder)
    source = git("rev-parse", "HEAD")
    if git("ls-remote", "origin", "refs/heads/main").partition("\t")[0] != source:
        raise ValueError("Push the built source to origin/main before publishing")
    ref = f"refs/tags/{tag}"
    refs = {
        name: sha for sha, name in (line.split("\t") for line in
                                  git("ls-remote", "origin", ref, f"{ref}^{{}}").splitlines())
    }
    if refs and refs.get(f"{ref}^{{}}", refs.get(ref)) != source:
        raise ValueError("The release tag already points to a different commit")
    token = access_token()
    url = f"{API}/tags/{tag}"
    try:
        record = request(token, url)
    except HTTPError as error:
        if error.code != 404:
            raise
        record = next((item for item in request(token, f"{API}?per_page=100") if item["tag_name"] == tag), None)
    if record is None:
        record = request(token, API, "POST", metadata={
            "tag_name": tag,
            "name": title,
            "target_commitish": source,
            "body": f"{notes.strip()}\n\n[Quellstand](https://github.com/{REPOSITORY}/commit/{source})",
            "draft": True,
            "prerelease": prerelease,
        })
    if record["tag_name"] != tag or record["prerelease"] != prerelease:
        raise ValueError("The existing release does not match the requested version or channel")
    if record["draft"] and record["target_commitish"] != source:
        raise ValueError("The existing draft targets a different commit")
    assets = {asset["name"]: asset for asset in record["assets"]}
    if not assets.keys() <= files.keys():
        raise ValueError("The existing release has unexpected assets")
    for name, (data, label) in files.items():
        asset = assets.get(name)
        if asset is None and record["draft"]:
            params = {"name": name}
            if label:
                params["label"] = label
            upload = record["upload_url"].split("{", 1)[0]
            asset = request(token, f"{upload}?{urlencode(params)}", "POST", payload=data)
        if asset is None:
            raise ValueError(f"The published release is missing {name}")
        validate_asset(asset, data, label)
        print(f"Verified {name}", flush=True)
    if record["draft"]:
        request(token, record["url"], "PATCH", metadata={"draft": False})
    result = request(token, url)
    assets = {asset["name"]: asset for asset in result["assets"]}
    if result["draft"] or result["prerelease"] != prerelease or assets.keys() != files.keys():
        raise ValueError("The published release does not contain the expected assets")
    for name, (data, label) in files.items():
        validate_asset(assets[name], data, label)
    print(result["html_url"], flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("notes", type=Path)
    parser.add_argument("--packages", type=Path, default=ROOT / "Releases",
                        help="Folder containing the verified release index and all packages")
    args = parser.parse_args()
    release = json.loads((ROOT / "release.json").read_text(encoding="utf-8"))
    publish(release, args.notes.read_text(encoding="utf-8"), args.packages)
