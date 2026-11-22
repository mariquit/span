document.getElementById('search').addEventListener('submit', async (event) => {
  event.preventDefault();
  const status = document.getElementById('status');
  const results = document.getElementById('results');
  results.textContent = '';
  status.textContent = 'Searching…';
  try {
    const response = await fetch('/search', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${document.getElementById('token').value}`,
      },
      body: JSON.stringify({
        scope: document.getElementById('scope').value,
        query: document.getElementById('query').value,
      }),
    });
    if (!response.ok) throw new Error(`Search failed: HTTP ${response.status}`);
    results.textContent = JSON.stringify(await response.json(), null, 2);
    status.textContent = 'Search complete.';
  } catch (error) {
    status.textContent = error.message;
  }
});
