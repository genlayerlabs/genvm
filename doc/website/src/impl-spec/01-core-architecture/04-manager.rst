Manager
=======

The GenVM :term:`Manager` is a HTTP server that provides an API for managing GenVM instances, modules, and related operations.

HTTP API Endpoints
------------------

Status and Health
~~~~~~~~~~~~~~~~~

.. http:get:: /status

   Get the current status of the manager and its modules.

   :>json object llm_module: Status of the LLM module
   :>json object web_module: Status of the web module

   **Example response**:

   .. code-block:: json

      {
        "llm_module": "running",
        "web_module": "stopped"
      }

Module Management
~~~~~~~~~~~~~~~~~

.. http:post:: /module/start

   Start a module with the specified configuration.

   :<json string module_type: Type of module to start (e.g., "llm", "web")
   :<json object config: Module-specific configuration

   :>json string result: Result status ("module_started")

   **Example request**:

   .. code-block:: json

      {
        "module_type": "llm",
        "config": {
          "host": "localhost",
          "port": 8080
        }
      }

.. http:post:: /module/stop

   Stop a running module.

   :<json string module_type: Type of module to stop

   :>json string result: Result status ("module_stopped" or "module_not_running")

   **Example request**:

   .. code-block:: json

      {
        "module_type": "llm"
      }

GenVM Execution
~~~~~~~~~~~~~~~

.. http:post:: /genvm/run

   Start a new GenVM instance for contract execution.

   :<json int major: Major version specification
   :<json object message: Contract execution message
   :<json bool is_sync: Whether execution is synchronous
   :<json bool capture_output: Whether to capture execution output
   :<json int max_execution_minutes: Maximum execution time in minutes
   :<json string host_data: Host-specific data as JSON string
   :<json string timestamp: Execution timestamp in RFC3339 format
   :<json string host: Host identifier
   :<json array extra_args: Additional arguments for execution

   :>json string result: Result status ("started")
   :>json int id: Unique identifier for the GenVM instance

   **Example response**:

   .. code-block:: json

      {
        "result": "started",
        "id": 12345
      }

.. http:post:: /genvm/run/readonly

   Execute a contract in read-only mode (not yet implemented).

   :reqheader Deployment-Timestamp: Contract deployment timestamp in RFC3339 format

   :<body: Contract bytecode

   :>json string schema: Contract schema information

.. http:get:: /genvm/(int:genvm_id)

   Get the status of a specific GenVM instance.

   :param genvm_id: Unique identifier of the GenVM instance

   :>json int genvm_id: The GenVM instance ID
   :>json object status: Current status of the GenVM instance

.. http:delete:: /genvm/(int:genvm_id)

   Gracefully shutdown a GenVM instance.

   :param genvm_id: Unique identifier of the GenVM instance
   :query int wait_timeout_ms: Timeout in milliseconds to wait for graceful shutdown (default: 30000)

   :>json string result: Result status ("shutdown_completed")
   :>json int genvm_id: The GenVM instance ID

   **Error response**:

   .. code-block:: json

      {
        "error": "timeout during shutdown",
        "genvm_id": 12345
      }

Contract Operations
~~~~~~~~~~~~~~~~~~~

.. http:post:: /contract/detect-version

   Detect the major version specification from contract bytecode.

   :reqheader Deployment-Timestamp: Contract deployment timestamp in RFC3339 format

   :<body: Contract bytecode

   :>json int specified_major: Detected major version

.. http:post:: /contract/pre-deploy-writes

   Generate storage writes required for contract deployment.

   :reqheader Deployment-Timestamp: Contract deployment timestamp in RFC3339 format

   :<body: Contract bytecode

   :>json array writes: Array of storage write operations

   **Example response**:

   .. code-block:: json

      {
        "writes": [
          ["<base64-encoded-key>", "<base64-encoded-value>"],
          ["<base64-encoded-key>", "<base64-encoded-value>"]
        ]
      }

Configuration Management
~~~~~~~~~~~~~~~~~~~~~~~~

.. http:post:: /log/level

   Set the logging level for the manager.

   :<json string level: Log level ("trace", "debug", "info", "warn", "error")

   :>json string result: Result status ("log_level_set")
   :>json string level: The new log level

   **Example request**:

   .. code-block:: json

      {
        "level": "debug"
      }

.. http:post:: /manifest/reload

   Reload the executor version manifest.

   :>json string result: Result status ("manifest_reloaded")

.. http:post:: /env

   Set an environment variable in the manager process.

   :<json string key: Environment variable name
   :<json string value: Environment variable value

   :>json string result: Result status ("env_var_set")
   :>json string key: The environment variable name

   **Example request**:

   .. code-block:: json

      {
        "key": "DEBUG_MODE",
        "value": "true"
      }

Resource Management
~~~~~~~~~~~~~~~~~~~

.. http:get:: /permits

   Get the current number of execution permits available.

   :>json int permits: Number of available permits

.. http:post:: /permits

   Set the number of execution permits.

   :<json int permits: New number of permits to allocate

   :>json string result: Result status ("permits_set")
   :>json int permits: The new number of permits

   **Example request**:

   .. code-block:: json

      {
        "permits": 10
      }

LLM Testing
~~~~~~~~~~~

.. http:post:: /llm/check

   Test availability and functionality of LLM provider configurations.

   :<json array configs: Array of LLM provider configurations
   :<json array test_prompts: Array of test prompts to execute

   :>json array results: Array of availability test results

   **Example request**:

   .. code-block:: json

      {
        "configs": [
          {
            "host": "https://api.openai.com",
            "provider": "openai-compatible",
            "model": "gpt-4o",
            "key": "${ENV[OPENAIKEY]}",
          }
        ],
        "test_prompts": [
          {
            "system_message": null,
            "user_message": "Respond with exactly two letters 'OK' and nothing else",
            "temperature": 0.2,
            "images": [],
            "max_tokens": 200,
            "use_max_completion_tokens": true
          }
        ]
      }

   **Example response**:

   .. code-block:: json

      [
        {
          "config_index": 0,
          "prompt_index": 0,
          "available": true,
          "error": null,
          "response": "Hello! How can I help you today?"
        }
      ]

Error Responses
---------------

All endpoints may return error responses in the following format:

.. code-block:: json

   {
     "error": "Description of the error"
   }

HTTP status codes used:

- ``200 OK``: Successful operation
- ``404 Not Found``: Endpoint not found
- ``500 Internal Server Error``: Server error or request processing failure
