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

import Constants from '../../services/constants';

let selectedInfoPanelDeo = Constants.infoPanelBasicDeo;

const settingsTabs = document.querySelectorAll('.tabs > li');
settingsTabs.forEach(clickedTab => {
    clickedTab.addEventListener('click', () => {
        settingsTabs.forEach(tab => {
            tab.classList.remove('active');
        });
        clickedTab.classList.add('active');
        selectedInfoPanelDeo.classList.add('display-none');
        switch (clickedTab.id) {
        case 'panel-basic':
            selectedInfoPanelDeo = Constants.infoPanelBasicDeo;
            break;
        case 'panel-advanced':
            selectedInfoPanelDeo = Constants.infoPanelAdvancedDeo;
            break;
        default:
            console.error('Unknown clicked tab: ' + clickedTab.id);
            break;
        }
        selectedInfoPanelDeo.classList.remove('display-none');
    });
});