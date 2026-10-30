<script>
  import { onMount } from 'svelte';

  let canvas;
  let drawing = false;
  let hasInk = false;
  let prediction = null;
  let probabilities = Array(10).fill(0);
  let status = 'Draw a digit to begin';
  let timer;
  let requestId = 0;

  const size = 280;
  const digits = Array.from({ length: 10 }, (_, i) => i);

  onMount(() => {
    clearCanvas();
    return () => clearTimeout(timer);
  });

  function context() {
    return canvas.getContext('2d', { willReadFrequently: true });
  }

  function clearCanvas() {
    const ctx = context();
    ctx.fillStyle = '#080b12';
    ctx.fillRect(0, 0, size, size);
    hasInk = false;
    prediction = null;
    probabilities = Array(10).fill(0);
    status = 'Draw a digit to begin';
    clearTimeout(timer);
    requestId++;
  }

  function point(event) {
    const rect = canvas.getBoundingClientRect();
    return {
      x: (event.clientX - rect.left) * size / rect.width,
      y: (event.clientY - rect.top) * size / rect.height
    };
  }

  function start(event) {
    event.preventDefault();
    canvas.setPointerCapture(event.pointerId);
    drawing = true;
    const { x, y } = point(event);
    const ctx = context();
    ctx.strokeStyle = '#fff';
    ctx.lineWidth = 24;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.lineTo(x + 0.01, y + 0.01);
    ctx.stroke();
    hasInk = true;
    schedulePrediction();
  }

  function move(event) {
    if (!drawing) return;
    event.preventDefault();
    const { x, y } = point(event);
    const ctx = context();
    ctx.lineTo(x, y);
    ctx.stroke();
    schedulePrediction();
  }

  function end() {
    if (!drawing) return;
    drawing = false;
    schedulePrediction(30);
  }

  function toPixels() {
    const source = context().getImageData(0, 0, size, size).data;
    let minX = size, minY = size, maxX = -1, maxY = -1;
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        if (source[(y * size + x) * 4] > 20) {
          minX = Math.min(minX, x); maxX = Math.max(maxX, x);
          minY = Math.min(minY, y); maxY = Math.max(maxY, y);
        }
      }
    }
    if (maxX < 0) return null;

    // MNIST uses a white digit on black, fitted inside a 28×28 image.
    const width = maxX - minX + 1;
    const height = maxY - minY + 1;
    const scale = 20 / Math.max(width, height);
    const target = document.createElement('canvas');
    target.width = 28; target.height = 28;
    const ctx = target.getContext('2d', { willReadFrequently: true });
    ctx.fillStyle = '#000'; ctx.fillRect(0, 0, 28, 28);
    ctx.drawImage(canvas, minX, minY, width, height,
      (28 - width * scale) / 2, (28 - height * scale) / 2,
      width * scale, height * scale);
    const rgba = ctx.getImageData(0, 0, 28, 28).data;
    return Array.from({ length: 784 }, (_, i) => rgba[i * 4] / 255);
  }

  function schedulePrediction(delay = 160) {
    clearTimeout(timer);
    requestId++;
    status = 'Reading your drawing…';
    timer = setTimeout(predict, delay);
  }

  async function predict() {
    if (!hasInk) return;
    const pixels = toPixels();
    if (!pixels) return;
    const current = ++requestId;
    status = 'Recognising…';
    try {
      const response = await fetch('/api/predict', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pixels })
      });
      const data = await response.json();
      if (!response.ok) throw new Error(data.error || 'Prediction failed');
      if (current !== requestId) return;
      prediction = data.digit;
      probabilities = data.probabilities;
      status = 'Live prediction';
    } catch (error) {
      if (current !== requestId) return;
      status = `Connection error: ${error.message}`;
    }
  }
</script>

<svelte:head>
  <meta name="description" content="Draw a digit and watch a pretrained ONNX model recognise it live with Rust, Burn and Axum." />
</svelte:head>

<div class="page">
  <header class="topbar">
    <div class="brand"><span class="brand-mark">✳</span><span>ink<span class="brand-dot">.</span></span></div>
    <span class="top-note"><span class="online-dot"></span> BURN + AXUM + SVELTE</span>
  </header>

  <main>
    <div class="intro">
      <div class="eyebrow"><span class="eyebrow-line"></span> MACHINE LEARNING, IN REAL TIME</div>
      <h1>Draw a number.<br /><em>Watch it think.</em></h1>
      <p>Sketch any digit from 0 to 9. A pretrained neural network recognises your handwriting as you draw.</p>
    </div>

    <div class="workspace">
      <section class="draw-panel" aria-label="Drawing area">
        <div class="panel-header"><span>01 / INPUT</span><span class="panel-label">YOUR CANVAS</span></div>
        <div class="canvas-wrap">
          <canvas bind:this={canvas} width="280" height="280" aria-label="Draw a digit here"
            onpointerdown={start} onpointermove={move} onpointerup={end} onpointercancel={end}></canvas>
          {#if !hasInk}<div class="canvas-hint" aria-hidden="true">DRAW HERE <span>↗</span></div>{/if}
        </div>
        <div class="panel-footer"><span>Use your mouse or finger</span><button class="clear-button" onclick={clearCanvas}>↺ &nbsp; Clear canvas</button></div>
      </section>

      <section class="result-panel" aria-label="Recognition results">
        <div class="panel-header"><span>02 / OUTPUT</span><span class="panel-label">MODEL RESULT</span></div>
        <div class="prediction-block">
          <div class="status"><span class:active={prediction !== null} class="status-dot"></span>{status}</div>
          <div class:empty={prediction === null} class="prediction-digit">{prediction === null ? '?' : prediction}</div>
          <div class="prediction-caption">{prediction === null ? 'AWAITING INPUT' : 'PREDICTED DIGIT'}</div>
        </div>
        <div class="probabilities">
          <div class="prob-title"><span>ALL DIGITS</span><span>CONFIDENCE</span></div>
          {#each digits as digit}
            <div class:chosen={prediction === digit} class="prob-row">
              <span class="prob-digit">{digit}</span>
              <div class="bar-track"><div class="bar-fill" style:width={`${(probabilities[digit] * 100).toFixed(1)}%`}></div></div>
              <span class="prob-value">{Math.round(probabilities[digit] * 100)}%</span>
            </div>
          {/each}
        </div>
      </section>
    </div>

    <div class="footnote"><span class="footnote-icon">✦</span> POWERED BY A PRETRAINED MNIST ONNX MODEL · INFERENCE RUNS ON THE RUST SERVER</div>
  </main>
</div>
