// Define a node struct for the linked list
pub struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

// Define the Queue struct
pub struct Queue<T> {
    head: Option<Box<Node<T>>>,
    tail: *mut Node<T>,
}

impl<T> Queue<T> {
    // Create a new empty queue
   pub fn new() -> Self {
        Queue {
            head: None,
            tail: std::ptr::null_mut(),
        }
    }

    // Check if the queue is empty
   pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    // Add an element to the back of the queue
    pub fn enqueue(&mut self, value: T) {
        let new_tail = Box::new(Node {
            value,
            next: None,
        });

        let raw_tail: *mut _ = Box::into_raw(new_tail);

        unsafe {
            if !self.tail.is_null() {
                (*self.tail).next = Some(Box::from_raw(raw_tail));
            } else {
                self.head = Some(Box::from_raw(raw_tail));
            }
            self.tail = raw_tail;
        }
    }


    // Remove and return the element at the front of the queue, along with the new head
   pub fn dequeue(&mut self) -> Option<(T, Option<&T>)> {
        self.head.take().map(move |mut old_head| {
            self.head = old_head.next.take();
            let new_head = self.head.as_ref().map(|node| &node.value);
            if self.head.is_none() {
                self.tail = std::ptr::null_mut();
            }
            (old_head.value, new_head)
        })
    }
}

