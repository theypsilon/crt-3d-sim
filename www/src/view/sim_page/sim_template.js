/* Copyright (c) 2019 José manuel Barroso Galindo <theypsilon@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>. */

import { html, render } from 'lit-html';
import { ifDefined } from 'lit-html/directives/if-defined';

const css = require('!css-loader!./css/sim_page.css').toString();

export function renderTemplate (state, view, root) {
    render(generateSimTemplate(state, view), root);
}

function generateSimTemplate (state, view) {
    return html`
        <style>
            ${css}
        </style>
        <canvas id="gl-canvas-id" tabindex=0></canvas>
        <div id="simulation-ui">
            <div id="fps-counter">${state.fps}</div>
            ${state.menu.visible ? html`
                <div id="info-panel">
                    ${state.menu.open ? html`
                        <div id="info-panel-content">
                            ${state.menu.entries.map(entry => generateTemplateFromGenericEntry(view, entry))}
                        </div>
                        ` : ''}
                    <div id="info-panel-toggle" 
                        class="collapse-button collapse-controller" 
                        @click="${() => view.toggleControls()}">${state.menu.controlsText}</div>
                </div>
            ` : ''}
        </div>
    `;
}

function generateTemplateFromGenericEntry (view, entry) {
    switch (entry.type) {
    case 'menu': return generateTemplateFromMenu(view, entry);
    case 'preset-buttons': return generateTemplateFromPresetButtons(view, entry);
    case 'button-input': return generateTemplateFromButtonInput(view, entry);
    case 'selectors-input': return generateTemplateFromSelectorsInput(view, entry);
    case 'number-input': return generateTemplateFromNumberInput(view, entry);
    case 'color-input': return generateTemplateFromColorInput(view, entry);
    case 'camera-input': return generateTemplateFromCameraInput(view, entry);
    default: throw new Error('Entry type ' + entry.type + ' not handled.');
    }
}

function generateTemplateFromMenu (view, menu) {
    return html`
        <div class="collapse-button collapse-top-menu ${menu.open ? 'not-collapsed' : 'collapsed'}" @click="${() => view.toggleMenu(menu)}">${menu.text}</div>
        ${menu.open ? html`
            <div class="info-category">
                ${menu.entries.map(entry => generateTemplateFromGenericEntry(view, entry))}
            </div>
        ` : ''}
    `;
}

function generateTemplateFromPresetButtons (view, presetButtons) {
    return html`
        <div class="preset-list ${presetButtons.class}">
            ${presetButtons.ref.choices.map(choices => html`
                <a class="btn preset-btn ${presetButtons.ref.selected === choices.preset ? 'active-preset' : ''}" data-preset="${choices.preset}" href="#"
                    @click="${() => view.clickPreset(choices.preset)}"
                    >${choices.text}</a>
            `)}
        </div>
    `;
}

function generateTemplateFromButtonInput (view, buttonInput) {
    return html`
        <div class="menu-entry menu-button ${buttonInput.class}" @click="${() => view.dispatchKey('keydown', buttonInput.ref.eventKind)}">
            <div class="feature-pack">
                <div class="feature-name">${buttonInput.text}</div>
            </div>
            <div class="feature-value input-holder"><div></div></div>
        </div>
    `;
}

function generateTemplateFromSelectorsInput (view, selectorInput) {
    return html`
        <div class="menu-entry ${selectorInput.class}">
            <div class="feature-pack">
                <div class="feature-name">${selectorInput.text}</div>
                <div class="feature-hotkeys">
                    <sup class="hotkey hk-inc" title="Press '${selectorInput.hk.inc}' to increse the value of this field">${selectorInput.hk.inc}</sup>
                    <sup class="hotkey hk-dec" title="Press '${selectorInput.hk.inc}' to decrease the value of this field">${selectorInput.hk.dec}</sup>
                </div>
            </div>
            <div class="feature-value input-holder">
                <div class="selector-inc"
                    @mouseup="${e => { e.preventDefault(); view.dispatchKey('keyup', selectorInput.ref.eventKind + '-inc'); }}"
                    @mousedown="${e => { e.preventDefault(); view.dispatchKey('keydown', selectorInput.ref.eventKind + '-inc'); }}"
                    >
                    <button class="button-inc-selector"
                        >↑</button>
                    <input class="number-input feature-readonly-input" type="text"
                        title="${ifDefined(selectorInput.ref.title)}"
                        .value="${selectorInput.ref.value}"
                        >
                </div>
                <button class="button-inc-dec"
                    @mouseup="${() => view.dispatchKey('keyup', selectorInput.ref.eventKind + '-dec')}"
                    @mousedown="${() => view.dispatchKey('keydown', selectorInput.ref.eventKind + '-dec')}"
                    >↓</button>
            </div>
        </div>
    `;
}

function generateTemplateFromNumberInput (view, numberInput) {
    return html`
        <div class="menu-entry ${numberInput.class}">
            <div class="feature-pack">
                <div class="feature-name">${numberInput.text}</div>
                <div class="feature-hotkeys">
                    <sup class="hotkey hk-inc" title="Press '${numberInput.hk.inc}' to increse the value of this field">${numberInput.hk.inc}</sup>
                    <sup class="hotkey hk-dec" title="Press '${numberInput.hk.inc}' to decrease the value of this field">${numberInput.hk.dec}</sup>
                </div>
            </div>
            <div class="feature-value input-holder">
                <button class="button-inc-dec"
                    @mouseup="${() => view.dispatchKey('keyup', numberInput.ref.eventKind + '-inc')}"
                    @mousedown="${() => view.dispatchKey('keydown', numberInput.ref.eventKind + '-inc')}"
                    >↑</button>
                <input class="number-input feature-modificable-input" type="number" 
                    placeholder="${numberInput.placeholder}" step="${numberInput.step}" min="${numberInput.min}" max="${numberInput.max}" .value="${numberInput.ref.value}"
                    @focus="${() => view.dispatchKey('keydown', 'input_focused')}"
                    @blur="${() => view.dispatchKey('keyup', 'input_focused')}"
                    @keypress="${e => e.charCode === 13 /* ENTER */ && e.target.blur()}"
                    @change="${e => view.changeSyncedInput(e.target.value, numberInput.ref.eventKind)}"
                    >
                <button class="button-inc-dec"
                    @mouseup="${() => view.dispatchKey('keyup', numberInput.ref.eventKind + '-dec')}"
                    @mousedown="${() => view.dispatchKey('keydown', numberInput.ref.eventKind + '-dec')}"
                    >↓</button>
            </div>
        </div>
    `;
}

function generateTemplateFromColorInput (view, colorInput) {
    return html`
        <div class="menu-entry ${colorInput.class}">
            <div class="feature-pack">
                <div class="feature-name">${colorInput.text}</div>
            </div>
            <div class="feature-value input-holder">
                <input class="feature-button" type="color" .value="${colorInput.ref.value}"
                    @change="${e => view.changeSyncedInput(parseInt('0x' + e.target.value.substring(1)), colorInput.ref.eventKind)}"
                    >
            </div>
        </div>
    `;
}

function generateTemplateFromCameraInput (view, cameraInput) {
    return html`
        <div class="menu-dual-entry-container">
            <div class="menu-dual-entry-item menu-dual-entry-1 ${cameraInput.class}">
                <div class="feature-name">Translation</div>
                <div id="feature-camera-movements" class="arrows-grid ${cameraInput.ref.free ? 'arrows-grid-move-free' : 'arrows-grid-move-lock'}">
                    <div></div><div class="input-cell">${generateTemplateArrowKey(view, 'W')}</div><div></div><div></div><div class="input-cell">${cameraInput.ref.free ? generateTemplateArrowKey(view, 'Q') : ''}</div>
                    <div class="input-cell">${generateTemplateArrowKey(view, 'A')}</div><div class="input-cell">${generateTemplateArrowKey(view, 'S')}</div><div class="input-cell">${generateTemplateArrowKey(view, 'D')}</div><div></div><div>${cameraInput.ref.free ? generateTemplateArrowKey(view, 'E') : ''}</div>
                </div>
            </div>
            <div class="menu-dual-entry-item menu-dual-entry-2">
                <div class="feature-name">Rotation</div>
                <div id="feature-camera-turns" class="arrows-grid arrows-grid-turn">
                        <div></div><div>${generateTemplateArrowKey(view, '↑')}</div><div></div><div></div><div>${generateTemplateArrowKey(view, '+')}</div><div class="rotator">⟳</div>
                        <div>${generateTemplateArrowKey(view, '←')}</div><div>${generateTemplateArrowKey(view, '↓')}</div><div>${generateTemplateArrowKey(view, '→')}</div><div></div><div>${generateTemplateArrowKey(view, '-')}</div><div class="rotator">⟲</div>
                </div>
            </div>
        </div>
        <div class="camera-matrix input-holder">
            <div class="matrix-row ${cameraInput.class}"></div><div class="matrix-top-row"><label class="text-center">X</label></div><div class="matrix-top-row"><label class="text-center">Y</label></div><div class="matrix-top-row"><label class="text-center">Z</label></div>
            <div class="matrix-row ${cameraInput.class}"><div class="matrix-row-head">positon</div></div>
                ${[cameraInput.ref.pos.x, cameraInput.ref.pos.y, cameraInput.ref.pos.z].map(ref => generateTemplateForCameraMatrixInput(view, ref))}
            <div class="matrix-row ${cameraInput.class}"><div class="matrix-row-head">direction</div></div>
                ${[cameraInput.ref.dir.x, cameraInput.ref.dir.y, cameraInput.ref.dir.z].map(ref => generateTemplateForCameraMatrixInput(view, ref))}
            <div class="matrix-row ${cameraInput.class}"><div class="matrix-row-head">axis up</div></div>
                ${[cameraInput.ref.axis_up.x, cameraInput.ref.axis_up.y, cameraInput.ref.axis_up.z].map(ref => generateTemplateForCameraMatrixInput(view, ref))}
        </div>
    `;
}

function generateTemplateArrowKey (view, key) {
    return html`
        <input type="button" class="activate-button feature-modificable-input" value="${key}"
            @mousedown="${() => view.dispatchKey('keydown', key.toLowerCase())}"
            @mouseup="${() => view.dispatchKey('keyup', key.toLowerCase())}"
        >
    `;
}

function generateTemplateForCameraMatrixInput (view, ref) {
    return html`
        <div class="input-cell">
            <input class="feature-modificable-input" type="number" step="0.01" .value="${ref.value}"
                @change="${e => view.changeSyncedInput(+e.target.value, ref.eventKind)}"
                @focus="${() => view.dispatchKey('keydown', 'input_focused')}"
                @blur="${() => view.dispatchKey('keyup', 'input_focused')}"
                @keypress="${e => e.charCode === 13 /* ENTER */ && e.target.blur()}"
                >
        </div>
    `;
}