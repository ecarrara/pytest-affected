import pytest
from _pytest.runner import runtestprotocol
from .store import RunInstance, SQLiteStore
from ._lib import Tracer, Murmur3Hasher


class Affected:
    def __init__(self):
        self.store: SQLiteStore = SQLiteStore()
        self.hasher = Murmur3Hasher()
        self.runs: list[RunInstance] = []

    def clear_cache(self, cache_ttl: int):
        self.store.delete_old_runs(cache_ttl)


@pytest.hookimpl()
def pytest_addoption(parser):
    group = parser.getgroup("affected")

    group.addoption(
        "--affected-min-time",
        action="store",
        type=int,
        help="Only cache results from tests that took more than N seconds.",
        default=0,
    )
    group.addoption(
        "--affected-cache-ttl",
        action="store",
        type=int,
        default=86400,
    )
    group.addoption(
        "--affected-cache-failures",
        action="store_true",
    )


@pytest.hookimpl(hookwrapper=True)
def pytest_sessionstart(session):
    session._affected = Affected()
    session._affected.clear_cache(session.config.option.affected_cache_ttl)
    yield


@pytest.hookimpl(hookwrapper=True)
def pytest_sessionfinish(session):
    affected: Affected = session._affected
    affected_min_time = session.config.option.affected_min_time
    affected_cache_failures = session.config.option.affected_cache_failures

    cacheable_runs = []
    for run in affected.runs:
        if run.call_duration > affected_min_time:
            if run.failed and not affected_cache_failures:
                continue
            cacheable_runs.append(run)

    affected.store.save_runs(cacheable_runs)

    yield


def search_cached_result_in_recent_runs(hasher, runs: list[RunInstance]):
    for run in runs:
        expected_files = set((filepath, hash) for filepath, hash in run.files)
        found_files = set(
            (filepath, hasher.hash_file(filepath)) for filepath, _ in run.files
        )
        if expected_files == found_files:
            return run

    return None


def pytest_runtest_protocol(item, nextitem):
    affected: Affected = item.session._affected

    runs = affected.store.list_recent_runs(node_id=item.nodeid, limit=50)
    found_run = search_cached_result_in_recent_runs(affected.hasher, runs)

    if found_run:
        item.ihook.pytest_runtest_logstart(nodeid=item.nodeid, location=item.location)
        for report in found_run.reports:
            item.ihook.pytest_runtest_logreport(report=report)
        item.ihook.pytest_runtest_logfinish(nodeid=item.nodeid, location=item.location)
    else:
        item.ihook.pytest_runtest_logstart(nodeid=item.nodeid, location=item.location)

        Tracer.clear_files()
        Tracer.start()
        reports = runtestprotocol(item=item, log=False, nextitem=nextitem)
        Tracer.stop()

        setup_duration, call_duration, teardown_duration = -1, -1, -1
        for report in reports:
            item.ihook.pytest_runtest_logreport(report=report)
            if report.when == "setup":
                setup_duration = report.duration
            if report.when == "call":
                call_duration = report.duration
            if report.when == "setup":
                setup_duration = report.duration
        item.ihook.pytest_runtest_logfinish(nodeid=item.nodeid, location=item.location)

        files = []
        for filepath in Tracer.user_files():
            hash = affected.hasher.hash_file(filepath)
            files.append((filepath, hash))

        affected.runs.append(
            RunInstance(
                nodeid=item.nodeid,
                files=files,
                reports=reports,
                setup_duration=setup_duration,
                call_duration=call_duration,
                teardown_duration=teardown_duration,
            )
        )

    return True
