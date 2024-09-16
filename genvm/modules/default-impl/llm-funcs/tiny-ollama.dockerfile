FROM ollama/ollama

ENV OLLAMA_HOST=127.0.0.1:4443

EXPOSE 4443

RUN sh -c 'ollama serve & sleep 3 ; ollama pull tinyllama'

COPY tiny-ollama.sh /tiny-ollama.sh

ENTRYPOINT ["/tiny-ollama.sh"]
