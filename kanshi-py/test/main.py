from kanshipy import KanshiPy, KanshiEvent
import time

kanshi = KanshiPy()

def onEvent(event: KanshiEvent):
  print(f"{event.event_type} at {event.target.path} ({event.target.kind})")

kanshi.watch("./test_dir")
kanshi.subscribe(onEvent)

kanshi.start()

time.sleep(10)

kanshi.close()