## Default Templates

<img align="center" width="300" src="default_templates.png" />

Followed the [$P Point-Cloud Recognizer](https://depts.washington.edu/acelab/proj/dollar/pdollar.html) documentation for this<br>

This one resolves the issues from the [unistroke-version](https://github.com/RefinedDev/unistroke-recognizer), also adds multistroke gestures.<br>

Performance-wise, it takes basically the same amount of time as the unistroke-version with half the number of points (32 instead of 64) and with more accuracy; the Greedy_5 algorithm has a time complexity of $O(n^{2+\epsilon})$<br>

*The milliseconds display in the web-build is inaccurate (it is also faster when ran on your system) and I am not sure why, probably because of the wasm32-unknown-unknown target*<br>
For a better experience build and run the project on your system.

## Controls

Draw with left mouse button or space bar; touch for touchscreen<br>
Recognize with right mouse button or the button on bottom right of your screen
