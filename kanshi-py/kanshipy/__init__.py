from typing import Callable
from ._kanshipy import KanshiPy as _Kanshipy
from ._kanshipy import KanshiEvent, KanshiEventTarget

class KanshiPy:
  
  _kanshi: _Kanshipy
  _callbacks: set[Callable[[KanshiEvent], None]]
  
  def __init__(self, force_engine: str | None = None):
    self._kanshi = _Kanshipy.new(force_engine=force_engine if force_engine else "")
    self._callbacks = set()
    
  def watch(self, dir: str):
    self._kanshi.watch(dir)
    
  def subscribe(self, callback: Callable[[KanshiEvent], None]):
    self._callbacks.add(callback)
  
  def _master_callback(self, event: KanshiEvent):
    for callback in self._callbacks:
      callback(event)
  
  def start(self):
    self._kanshi.start(self._master_callback)
  
  def close(self):
    self._kanshi.close()