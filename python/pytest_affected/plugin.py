import sys

from _pytest.runner import runtestprotocol

from ._lib import Murmur3Hasher, Tracer
from .store import RunInstance, SQLiteStore


class Affected:
    def __init__(self):
        self.store: SQLiteStore = SQLiteStore()
        self.hasher = Murmur3Hasher()


def pytest_sessionstart(session):
    session._affected = Affected()


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
        tracer = Tracer()
        sys.settrace(tracer.tracefunc)

        item.ihook.pytest_runtest_logstart(nodeid=item.nodeid, location=item.location)
        reports = runtestprotocol(item=item, log=False, nextitem=nextitem)
        for report in reports:
            item.ihook.pytest_runtest_logreport(report=report)
        item.ihook.pytest_runtest_logfinish(nodeid=item.nodeid, location=item.location)

        files = []
        for filepath in tracer.user_files:
            hash = affected.hasher.hash_file(filepath)
            files.append((filepath, hash))

        affected.store.save_run(
            RunInstance(
                nodeid=item.nodeid,
                files=files,
                reports=reports,
            )
        )

    return True
