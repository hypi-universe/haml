# Hypi Application Markup Language, HAML

The HAML language is designed to provide a descriptive generative AI friendly way to design and build applications.


## Generative AI

Gen AI is great at generating new content, not so much at editing existing ones.
The larger a file is for it to "edit" the more likely it will change it since it is not editing but regenerating new content and retaining the old one.
As such, HAML is designed such that all the major components can be freestanding which allows them to be imported where they're used.

Current sections which can be imported are:

* Table
* Endpoint
* Pipeline

This leaves any gen AI model to only have to generate the `schema.xml` which imports them. As this will be shorter, it improves the odds that a model retains the previous content when told to do so and only modify it by adding an import for anything new it generated.
