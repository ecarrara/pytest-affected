import pytest

from project1 import hello_world


@pytest.mark.parametrize("value", range(1000))
def test_pass(value):
    assert hello_world(2, 2) == 4
