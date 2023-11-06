# pytest-affected

![Build Status](https://img.shields.io/github/actions/workflow/status/ecarrara/pytest-affected/CI.yml)

pytest-affected is a pytest plugin that helps you optimize the execution of
pytest tests by caching and re-running only the tests that are affected by
changes in your codebase. This can significantly reduce the time required to
run your CI pipeline if your project have a lot of slow tests.

## Installation

pytest-affected plugin is available on PyPI:

```sh
pip install pytest-affected
```

## Usage

Add `pytest_affected.plugin` to `PYTEST_PLUGINS` environment variable to active this plugin.

```sh
PYTEST_PLUGINS=pytest_affected.plugin pytest
```

## License

pytest-affected is distributed under the MIT License. See LICENSE for more information.
