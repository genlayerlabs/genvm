Acquired from https://www.iana.org/domains/root/db with
```js
let table = document.getElementById('tld-table')
JSON.stringify(Array.from(table.rows).map(x => x.cells[0].innerText).filter(x => x.startsWith('.')).sort())
```
