// The story player: tokens move along the edges of the active step.
// The times in sd-data are at speed 1. A step plays once and stops.
(function () {
  "use strict";
  var data = JSON.parse(document.getElementById("sd-data").textContent);
  var stories = data.stories || [];
  var box = document.querySelector(".sd-player");
  if (!stories.length || !box) return;
  var svg = document.querySelector(".sd-view svg");
  var NS = "http://www.w3.org/2000/svg";
  var SPEEDS = [0.5, 1, 2];
  var STATUS = { 1: "done", 2: "in progress", 3: "planned" };
  var still = window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
  var pick = box.querySelector("select");
  var playBtn = box.querySelector("[data-p=play]");
  var speedBtn = box.querySelector("[data-p=speed]");
  var cap = box.querySelector(".sd-cap");
  var kick = svg.querySelector(".kk");
  var kickText = kick ? kick.textContent : "";
  var layer = document.createElementNS(NS, "g");
  svg.appendChild(layer);

  var si = 0, t = 0, until = null, playing = false, speed = 1, last = 0, beat = -1, shown = {}, orig = {}, active = false;

  stories.forEach(function (s, k) {
    var o = document.createElement("option");
    o.value = k;
    o.textContent = s.title;
    pick.appendChild(o);
  });

  function q(v) {
    return '"' + String(v).replace(/["\\]/g, "\\$&") + '"';
  }
  function edge(i) {
    return svg.querySelectorAll("[data-i=" + q(i) + "]");
  }
  function route(i) {
    var el = svg.querySelector("[data-i=" + q(i) + "] .rt");
    return el && el.getTotalLength ? el : null;
  }
  function nodes(id) {
    return svg.querySelectorAll("[data-k=node][data-id=" + q(id) + "]");
  }

  // A progress node: put the drawing of the status in place of the node.
  // Status 0 puts back the drawing of the page.
  function status(id, s) {
    if (shown[id] === s) return;
    var old = nodes(id);
    if (!old.length) return;
    if (!(id in orig)) {
      orig[id] = Array.prototype.map.call(old, function (el) {
        return el.outerHTML;
      }).join("");
    }
    var html = s === 0 ? orig[id] : (data.variants[id] || [])[s === 1 ? 2 : s === 2 ? 1 : 0];
    if (!html) return;
    old[old.length - 1].insertAdjacentHTML("afterend", html);
    old.forEach(function (el) {
      el.remove();
    });
    shown[id] = s;
  }

  function pulse(els) {
    els.forEach(function (el) {
      el.classList.remove("sd-pulse");
      void el.getBBox();
      el.classList.add("sd-pulse");
    });
  }

  function story() {
    return stories[si];
  }

  // The beat at time t, or -1 in the pause at the end. In a progress
  // story, the last beat that started.
  function beatAt(s, t) {
    var at = -1;
    for (var k = 0; k < s.beats.length; k++) {
      var b = s.beats[k];
      if (t >= b.t && t < b.t + b.d) return k;
      if (s.mode === "progress" && t >= b.t) at = k;
    }
    return at;
  }

  // A step stops 1 ms before its end, so the token stays in view.
  function phase(b) {
    return t < b.t ? 3 : t < b.t + b.d - 1 ? 2 : b.node.s;
  }

  function caption(s, k) {
    if (k < 0) return "";
    var b = s.beats[k];
    var head = "<b>" + (k + 1) + " / " + s.beats.length + "</b> ";
    if (b.node) return head + esc(b.node.id) + ": " + STATUS[phase(b)];
    return head + b.moves.map(function (m) {
      var r = edge(m.i)[0];
      var a = r ? r.dataset.from : "", z = r ? r.dataset.to : "";
      if (m.back) { var x = a; a = z; z = x; }
      var text = esc(a) + (m.hollow ? " --> " : " -> ") + esc(z);
      if (m.cap) text += " &middot; " + esc(m.cap);
      if (m.label && m.label !== m.cap) text += " <span>[" + esc(m.label) + "]</span>";
      return text;
    }).join(" &amp; ");
  }

  function esc(s) {
    return String(s).replace(/[&<>"]/g, function (c) {
      return "&#" + c.charCodeAt(0) + ";";
    });
  }

  function draw() {
    var s = story();
    var k = s.mode === "stream" ? 0 : beatAt(s, t);
    layer.textContent = "";
    svg.querySelectorAll(".sd-act").forEach(function (el) {
      el.classList.remove("sd-act");
    });
    svg.classList.toggle("sd-play", active && s.mode === "steps");
    if (!active) {
      cap.textContent = "";
      return;
    }
    if (s.mode === "progress" && active) {
      // The done count of the kicker follows the shown statuses.
      var m = /( \/ )(\d+)( of )/.exec(kickText), n = m ? +m[2] : 0;
      s.beats.forEach(function (b) {
        var p = phase(b), v = data.variants[b.node.id] || [];
        status(b.node.id, p);
        n += (p === 1) - (v[3] === 1);
      });
      if (m) kick.textContent = kickText.replace(m[0], m[1] + n + m[3]);
    }
    if (k !== beat) {
      var old = s.beats[beat];
      // The node at the end of a step pulses when the token arrives.
      if (old && s.mode === "steps") {
        old.moves.forEach(function (m) {
          var r = edge(m.i)[0];
          if (r) pulse(Array.prototype.slice.call(nodes(m.back ? r.dataset.from : r.dataset.to)));
        });
      }
      if (k >= 0 && s.beats[k].node) pulse(Array.prototype.slice.call(nodes(s.beats[k].node.id)));
      beat = k;
    }
    cap.innerHTML = s.mode === "stream" ? "<b>" + esc(s.title) + "</b> stream" : caption(s, k);
    if (k < 0 || s.mode === "progress") return;
    var b = s.beats[k];
    b.moves.forEach(function (m) {
      var p = route(m.i);
      if (!p) return;
      edge(m.i).forEach(function (el) {
        if (s.mode === "steps") el.classList.add("sd-act");
      });
      var phases = s.mode === "stream" ? [t % s.total, (t + s.total / 2) % s.total] : [t - b.t];
      var d = s.mode === "stream" ? s.total : b.d;
      phases.forEach(function (ph) {
        var u = still && s.mode === "steps" ? 1 : Math.min(ph / d, 1);
        if (m.back) u = 1 - u;
        var pt = p.getPointAtLength(u * p.getTotalLength());
        var c = document.createElementNS(NS, "circle");
        c.setAttribute("r", "5.5");
        c.setAttribute("cx", pt.x);
        c.setAttribute("cy", pt.y);
        c.setAttribute("class", "sd-tk" + (m.hollow ? " sd-h" : ""));
        layer.appendChild(c);
      });
    });
  }

  function setPlaying(on) {
    playing = on;
    playBtn.innerHTML = on ? "&#10074;&#10074;" : "&#9654;";
    playBtn.setAttribute("aria-label", on ? "Pause" : "Play");
    if (on) {
      last = performance.now();
      requestAnimationFrame(tick);
    }
  }

  function tick(now) {
    if (!playing) return;
    var s = story();
    t += (now - last) * speed;
    last = now;
    if (until !== null && t >= until) {
      t = until;
      until = null;
      setPlaying(false);
    } else if (t >= s.total) {
      t %= s.total;
    }
    draw();
    if (playing) requestAnimationFrame(tick);
  }

  // Puts each progress node back to the drawing of the page.
  function restore() {
    Object.keys(orig).forEach(function (id) {
      status(id, 0);
    });
    if (kick) kick.textContent = kickText;
  }

  // Plays one beat and stops at its end. With reduced motion, it jumps.
  function step(d) {
    var s = story();
    if (s.mode === "stream") return;
    var k = active ? beatAt(s, t) : -1;
    active = true;
    if (k < 0) k = d > 0 ? -1 : s.beats.length;
    if (until !== null && d > 0) k = beatAt(s, until - 1);
    k = Math.min(Math.max(k + d, 0), s.beats.length - 1);
    var b = s.beats[k];
    until = b.t + b.d - 1;
    if (still) {
      t = until;
      until = null;
      setPlaying(false);
      draw();
      return;
    }
    t = b.t;
    beat = k;
    draw();
    setPlaying(true);
  }

  function choose(k) {
    restore();
    si = k;
    t = 0;
    beat = -1;
    until = null;
    active = false;
    setPlaying(false);
    draw();
  }

  function toggle() {
    until = null;
    active = true;
    setPlaying(!playing);
  }

  pick.addEventListener("change", function () {
    choose(+pick.value);
  });
  box.addEventListener("click", function (e) {
    var b = e.target.closest("button");
    if (!b) return;
    var act = b.getAttribute("data-p");
    if (act === "play") toggle();
    else if (act === "back") step(-1);
    else if (act === "next") step(1);
    else if (act === "speed") {
      speed = SPEEDS[(SPEEDS.indexOf(speed) + 1) % SPEEDS.length];
      speedBtn.textContent = speed + "x";
    }
  });

  // The capture phase runs before the viewer, so the arrow keys step the
  // story and not the parts.
  window.addEventListener(
    "keydown",
    function (e) {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      var tag = e.target.tagName;
      if (tag === "SELECT" || tag === "INPUT" || (tag === "BUTTON" && e.key === " ")) return;
      if (e.target.closest && e.target.closest("[data-sd-st]")) return;
      if (e.key === " ") toggle();
      else if (e.key === "ArrowRight") step(1);
      else if (e.key === "ArrowLeft") step(-1);
      else return;
      e.preventDefault();
      e.stopPropagation();
    },
    true
  );

  choose(0);
  cap.textContent = "Space plays the story. The arrow keys step.";
})();
