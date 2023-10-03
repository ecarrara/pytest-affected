import time

from project1 import hello_world


def test_pass():
    time.sleep(3)
    assert hello_world(2, 2) == 4


def test_fail():
    time.sleep(3)
    assert hello_world(2, 2) == 5
