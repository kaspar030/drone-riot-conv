# drone-riot-conv

A custom conversion filter used by RIOT's Drone instance.

Currently, it looks for any "parallelism:" field. If present, it multiplies the
pipeline, appending "-<n>" to "name:".

If anything is not understood, the .drone.yml is passed through.
This also happens with Drone v0.8 syntax pipelines.
