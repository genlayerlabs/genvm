Greyboxing
==========

It refers to the technique of preventing attacks on LLMs. Implementing it is a responsibility of every node,
as bundled presets can be attacked.

Greyboxing can be achieved by a few methods:

#. Using different llms, potentially selected based on the request itself
#. Randomizing llm calling parameters
#. Modifying prompts

The LLM :term:`Module` provides greyboxing capabilities via lua scripting.

Retrieving Data from :term:`Host`
---------------------------------

Host can provide additional data to the :term:`Module` to help it make decisions,
only transaction id and node address are required, as they are required for signing requests.

Current Built-in Filters
------------------------

To simplify implementation, we provide a set of built-in filters
that can be used from the script

Text
^^^^

- Zero width character removal
- Whitespace normalization
- Unicode normalization

Image
^^^^^

- Unsharpen
- GuassianNoise
- JPEG reconversion

Example Usage
-------------


.. code-block:: lua

  args.prompt = lib.rs.filter_text(args.prompt, {
    'NFKC',
    'RmZeroWidth',
    'NormalizeWS'
  })

  args.images[0] = lib.rs.filter_image(args.images[0], {
    { Unsharpen = { 2.0, 4.0 } },
    { GaussianNoise = 0.05 },
    { JpegRecompress = 0.8 }
  })
