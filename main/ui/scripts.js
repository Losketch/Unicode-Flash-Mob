let fontList = [];
let singleFontPath = '';

function toggleStep(element, type) {
  if (type === 'radio') {
    // 对于单选，先清除同组的其他选择
    const group = element.getAttribute('data-group');
    if (group) {
      document.querySelectorAll(`[data-group="${group}"]`).forEach(card => {
        card.classList.remove('selected');
        // 隐藏相关控件
        const fontSelector = card.querySelector('#fontSelector');
        const unicodeRange = card.querySelector('#unicodeRange');
        if (fontSelector) fontSelector.style.display = 'none';
        if (unicodeRange) unicodeRange.style.display = 'none';
      });
    }

    // 选择当前项
    element.classList.add('selected');

    // 显示相关控件
    const fontSelector = element.querySelector('#fontSelector');
    const unicodeRange = element.querySelector('#unicodeRange');
    if (fontSelector) fontSelector.style.display = 'block';
    if (unicodeRange) unicodeRange.style.display = 'block';
  } else {
    // 复选框逻辑
    element.classList.toggle('selected');
  }

  generateCommands();
}

function handleFontFiles(input) {
  const files = input.files;
  if (files.length > 0) {
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const filePath = getFileFullPath(file);
      if (!fontList.includes(filePath)) {
        fontList.push(filePath);
      }
    }
    updateFontList();
    generateCommands();
  }
}

function handleSingleFontFile(input) {
  const file = input.files[0];
  if (file) {
    singleFontPath = getFileFullPath(file);
    updateSingleFontDisplay();
    generateCommands();
  }
}

function getFileFullPath(file) {
  // 浏览器安全限制，无法获取绝对路径，使用相对路径作为示例
  // 在实际使用中，用户需要手动输入完整路径
  return file.webkitRelativePath || file.name;
}

function updateSingleFontDisplay() {
  const display = document.getElementById('selectedSingleFont');
  if (singleFontPath) {
    display.className = 'selected-font';
    display.textContent = singleFontPath;
  } else {
    display.className = 'no-font-selected';
    display.textContent = '未选择字体文件';
  }
}

function updateFontList() {
  const container = document.getElementById('fontList');

  if (fontList.length === 0) {
    container.innerHTML = '<div class="empty-list">请选择字体文件</div>';
    return;
  }

  container.innerHTML = '';

  fontList.forEach((font, index) => {
    const item = document.createElement('div');
    item.className = 'font-item';
    item.innerHTML = `
                    <span class="font-name">${font}</span>
                    <div class="font-controls">
                        <button class="font-btn" onclick="moveFontUp(${index})" ${index === 0 ? 'disabled' : ''}>↑</button>
                        <button class="font-btn" onclick="moveFontDown(${index})" ${index === fontList.length - 1 ? 'disabled' : ''}>↓</button>
                        <button class="font-btn" onclick="removeFont(${index})" style="background: #ef4444;">删除</button>
                    </div>
                `;
    container.appendChild(item);
  });
}

function moveFontUp(index) {
  if (index > 0) {
    [fontList[index], fontList[index - 1]] = [fontList[index - 1], fontList[index]];
    updateFontList();
    generateCommands();
  }
}

function moveFontDown(index) {
  if (index < fontList.length - 1) {
    [fontList[index], fontList[index + 1]] = [fontList[index + 1], fontList[index]];
    updateFontList();
    generateCommands();
  }
}

function removeFont(index) {
  fontList.splice(index, 1);
  updateFontList();
  generateCommands();
}

function clearAll() {
  document.querySelectorAll('.step-card').forEach(card => {
    card.classList.remove('selected');
  });

  // 隐藏所有控件
  document.querySelectorAll('#fontSelector, #unicodeRange').forEach(el => {
    if (el) el.style.display = 'none';
  });

  fontList = [];
  singleFontPath = '';
  updateFontList();
  updateSingleFontDisplay();

  // 清空输入框
  document.getElementById('startHex').value = '';
  document.getElementById('endHex').value = '';
  document.getElementById('fontFileInput').value = '';
  document.getElementById('singleFontFileInput').value = '';

  generateCommands();
}

function generateCommands() {
  const selectedSteps = document.querySelectorAll('.step-card.selected');
  const output = document.getElementById('commandOutput');

  if (selectedSteps.length === 0) {
    output.textContent = '# 请选择要执行的步骤，然后点击"生成命令"';
    return;
  }

  let commands = [];
  commands.push('# Unicode Flash Mob 命令序列，此框内容可编辑');
  commands.push('# 生成时间: ' + new Date().toLocaleString('zh-CN'));
  commands.push('# 由于浏览器安全限制只能获取文件名，请在文件名前输入完整路径前缀（如 C:\\Fonts\\）');
  commands.push('');

  selectedSteps.forEach(step => {
    const stepNum = step.getAttribute('data-step');
    const stepTitle = step.querySelector('.step-title').textContent;

    commands.push(`# 步骤 ${stepNum}: ${stepTitle}`);

    switch (stepNum) {
      case '1':
        commands.push(step.querySelector('.step-command').textContent);
        break;
      case '2':
        commands.push(step.querySelector('.step-command').textContent);
        break;
      case '3':
        commands.push(step.querySelector('.step-command').textContent);
        break;
      case '4':
        commands.push(step.querySelector('.step-command').textContent);
        break;
      case '5':
        if (fontList.length > 0) {
          const fontPaths = fontList.map(font => `"${font}"`).join(' ');
          commands.push(`./unicode_flash_mob.exe extract ${fontPaths}`);
        } else {
          commands.push('./unicode_flash_mob.exe extract [请选择字体文件]');
        }
        break;
      case '6':
        const startHex = document.getElementById('startHex').value || '0000';
        const endHex = document.getElementById('endHex').value || '10FFFF';
        const fontPath = singleFontPath || '[请选择字体文件]';
        commands.push(`./unicode_flash_mob.exe generate-unicode-range --file combined_unicode_list.txt --start ${startHex} --end ${endHex} --font "${fontPath}"`);
        break;
      case '7':
        commands.push(step.querySelector('.step-command').textContent);
        break;
      case '8':
        commands.push(step.querySelector('.step-command').textContent);
        break;
      case '9':
        commands.push(step.querySelector('.step-command').textContent);
        break;
      case '10':
        commands.push(step.querySelector('.step-command').textContent);
        break;
      case '11':
        commands.push(step.querySelector('.step-command').textContent);
        break;
      case '12':
        commands.push(step.querySelector('.step-command').textContent);
        break;
        break;
    }
    commands.push('');
  });

  commands.push('# 执行完成');
  commands.push('Write-Host "所有选定步骤已完成" -ForegroundColor Green');
  commands.push('');
  commands.push('');

  output.textContent = commands.join('\n');
}

function copyToClipboard() {
  const output = document.getElementById('commandOutput');
  navigator.clipboard.writeText(output.textContent).then(() => {
    const btn = document.querySelector('.copy-btn');
    const originalText = btn.textContent;
    btn.textContent = '已复制!';
    btn.style.background = '#10b981';

    setTimeout(() => {
      btn.textContent = originalText;
      btn.style.background = '#3b82f6';
    }, 2000);
  }).catch(() => {
    // 如果浏览器不支持clipboard API，使用备用方法
    const textArea = document.createElement('textarea');
    textArea.value = output.textContent;
    document.body.appendChild(textArea);
    textArea.select();
    document.execCommand('copy');
    document.body.removeChild(textArea);

    const btn = document.querySelector('.copy-btn');
    const originalText = btn.textContent;
    btn.textContent = '已复制!';
    btn.style.background = '#10b981';

    setTimeout(() => {
      btn.textContent = originalText;
      btn.style.background = '#3b82f6';
    }, 2000);
  });
}

// 16进制输入验证
['startHex', 'endHex'].forEach(id => {
  const element = document.getElementById(id);
  if (element) {
    element.addEventListener('input', function(e) {
      // 只允许输入16进制字符
      this.value = this.value.replace(/[^0-9A-Fa-f]/g, '').toUpperCase();
      generateCommands();
    });
  }
});

// 初始化
updateFontList();
updateSingleFontDisplay();
generateCommands();

function toggleAllSections() {
  const sections = document.querySelectorAll('.section');
  const allCollapsed = Array.from(sections).every(section => section.classList.contains('collapsed'));

  sections.forEach(section => {
    section.classList.add('collapsible');
    if (allCollapsed) {
      section.classList.remove('collapsed');
    } else {
      section.classList.add('collapsed');
    }
  });
}

document.addEventListener('DOMContentLoaded', function() {
  const sections = document.querySelectorAll('.section');
  sections.forEach(section => {
    section.classList.add('collapsible');
    const title = section.querySelector('.section-title');
    title.addEventListener('click', function() {
      section.classList.toggle('collapsed');
    });
  });
});