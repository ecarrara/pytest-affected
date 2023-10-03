import sys

from _pytest.runner import runtestprotocol

from ._lib import Murmur3Hasher, Tracer
from .store import RunInstance, SQLiteStore


class Affected:
    def __init__(self):
        self.store: SQLiteStore = SQLiteStore()
        self.hasher = Murmur3Hasher()
        self.tracer = Tracer()
        self.runs = []


def pytest_sessionstart(session):
    session._affected = Affected()


def pytest_sessionfinish(session):
    affected = session._affected
    affected.store.save_runs(affected.runs)


def search_cached_result_in_recent_runs(hasher, runs: list[RunInstance]):
    for run in runs:
        expected_files = set((filepath, hash) for filepath, hash in run.files)
        found_files = set((filepath, hasher.hash_file(filepath)) for filepath, _ in run.files)
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

        affected.tracer.clear_files()
        sys.settrace(affected.tracer.tracefunc)
        reports = runtestprotocol(item=item, log=False, nextitem=nextitem)
        sys.settrace(None)

        for report in reports:
            item.ihook.pytest_runtest_logreport(report=report)
        item.ihook.pytest_runtest_logfinish(nodeid=item.nodeid, location=item.location)

        files = []
        for filepath in affected.tracer.user_files:
            hash = affected.hasher.hash_file(filepath)
            files.append((filepath, hash))

        affected.runs.append(
            RunInstance(
                nodeid=item.nodeid,
                files=files,
                reports=reports,
            )
        )

    return True
