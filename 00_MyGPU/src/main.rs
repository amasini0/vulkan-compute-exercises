use ash::{Entry, vk};

fn main() {}

/*
==================================== Task 0 ====================================
1) Load the the Vulkan entrypoint using ash's Entry::load() or Entry::linked().
2) Create a Vulkan instance. Give it an application name "Task 0", a version
number 1, and an engine name of "None". The API version should be 1.3.
3) Query your physical devices. Retrieve their properties and print out
- the total number of devices
- their names
- the latest API they support (human-readable! Might be a bit tricky!)
4) Physical devices have several types of property collections available. Out of
all of them, find three properties that should be relevant to compute jobs and
print them out.
5) Optional: you can go and explore a bit, checking out the individual features
of your devices and queue families it provides. What kinds does it support?
Check out an advanced feature using the pNext pointer of VkPhysicalDeviceFeatures2.
*/
