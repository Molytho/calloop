initSidebarItems({"enum":[["Mode","Possible modes for registering a file descriptor"],["PostAction","Possible actions that can be requested to the event loop by an event source once its events have been processed"]],"mod":[["channel","An MPSC channel whose receiving end is an event source"],["futures","A futures executor as an event source"],["generic","A generic event source wrapping an IO objects or file descriptor"],["io","Adapters for async IO objects"],["ping","Ping to the event loop"],["signals","Event source for tracking Unix signals"],["timer","Timer-based event sources"]],"struct":[["Dispatcher","An event source with its callback."],["EventLoop","An event loop"],["Idle","An idle callback that was inserted in this loop"],["InsertError","An error generated when trying to insert an event source"],["Interest","Interest to register regarding the file descriptor"],["LoopHandle","An handle to an event loop"],["LoopSignal","A signal that can be shared between thread to stop or wakeup a running event loop"],["Poll","The polling system"],["Readiness","Readiness for a file descriptor notification"],["RegistrationToken","A token representing a registration in the [`EventLoop`]."],["Token","A token (for implementation of the `EventSource` trait)"]],"trait":[["EventSource","Trait representing an event source"]]});