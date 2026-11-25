"""Compatibility entry point for the release publishing command."""
from runpy import run_module

if __name__ == "__main__":
    run_module("publish_release", run_name="__main__")
