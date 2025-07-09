// WasabiOS 技術解説ページ共通JavaScript

class TechExplanationPage {
    constructor() {
        this.currentStep = -1;
        this.totalSteps = 0;
        this.init();
    }

    init() {
        this.setupStepNavigation();
        this.setupKeyboardNavigation();
        this.countSteps();
    }

    setupStepNavigation() {
        // ステップ数をカウント
        this.countSteps();
        
        // ボタンイベントの設定
        const startBtn = document.getElementById('startBtn');
        const nextBtn = document.getElementById('nextBtn');
        const resetBtn = document.getElementById('resetBtn');

        if (startBtn) {
            startBtn.addEventListener('click', () => this.startDemo());
        }
        
        if (nextBtn) {
            nextBtn.addEventListener('click', () => this.nextStep());
        }
        
        if (resetBtn) {
            resetBtn.addEventListener('click', () => this.resetDemo());
        }
    }

    setupKeyboardNavigation() {
        document.addEventListener('keydown', (e) => {
            if (e.key === 'ArrowRight' || e.key === ' ') {
                e.preventDefault();
                if (this.currentStep === -1) {
                    this.startDemo();
                } else if (this.currentStep < this.totalSteps) {
                    this.nextStep();
                }
            } else if (e.key === 'ArrowLeft') {
                e.preventDefault();
                this.prevStep();
            } else if (e.key === 'r' || e.key === 'R') {
                e.preventDefault();
                this.resetDemo();
            } else if (e.key === 'Escape') {
                e.preventDefault();
                this.resetDemo();
            }
        });
    }

    countSteps() {
        const steps = document.querySelectorAll('.step');
        this.totalSteps = steps.length - 1; // 0-indexed
    }

    startDemo() {
        this.currentStep = 0;
        const nextBtn = document.getElementById('nextBtn');
        const startBtn = document.getElementById('startBtn');
        
        if (nextBtn) nextBtn.disabled = false;
        if (startBtn) startBtn.style.display = 'none';
        
        this.showStep(this.currentStep);
        this.updateProgress();
    }

    nextStep() {
        if (this.currentStep < this.totalSteps) {
            this.currentStep++;
            this.showStep(this.currentStep);
            this.updateProgress();
        }
        
        if (this.currentStep >= this.totalSteps) {
            const nextBtn = document.getElementById('nextBtn');
            if (nextBtn) nextBtn.disabled = true;
        }
    }

    prevStep() {
        if (this.currentStep > 0) {
            this.currentStep--;
            this.showStep(this.currentStep);
            this.updateProgress();
            
            const nextBtn = document.getElementById('nextBtn');
            if (nextBtn) nextBtn.disabled = false;
        }
    }

    showStep(step) {
        // 全ステップを非表示
        for (let i = 0; i <= this.totalSteps; i++) {
            const stepElement = document.getElementById(`step${i}`);
            if (stepElement) {
                stepElement.classList.remove('active');
            }
        }
        
        // 現在のステップを表示
        const currentStepElement = document.getElementById(`step${step}`);
        if (currentStepElement) {
            currentStepElement.classList.add('active');
            
            // ステップ固有の処理を実行
            this.executeStepSpecificActions(step);
            
            // スムーズスクロール
            currentStepElement.scrollIntoView({ 
                behavior: 'smooth', 
                block: 'start' 
            });
        }
    }

    executeStepSpecificActions(step) {
        // 各ステップで実行する特別な処理
        // 継承先でオーバーライドして使用
        const event = new CustomEvent('stepChanged', { 
            detail: { step: step } 
        });
        document.dispatchEvent(event);
    }

    resetDemo() {
        this.currentStep = -1;
        const nextBtn = document.getElementById('nextBtn');
        const startBtn = document.getElementById('startBtn');
        
        if (nextBtn) nextBtn.disabled = true;
        if (startBtn) {
            startBtn.style.display = 'inline-block';
        }
        
        // 全ステップを非表示
        for (let i = 0; i <= this.totalSteps; i++) {
            const stepElement = document.getElementById(`step${i}`);
            if (stepElement) {
                stepElement.classList.remove('active');
            }
        }
        
        this.updateProgress();
        
        // ページトップにスクロール
        window.scrollTo({ top: 0, behavior: 'smooth' });
    }

    updateProgress() {
        const progressBar = document.getElementById('progressBar');
        const progressText = document.getElementById('progressText');
        
        if (progressBar) {
            const progress = this.currentStep === -1 ? 0 : 
                           ((this.currentStep + 1) / (this.totalSteps + 1)) * 100;
            progressBar.style.width = `${progress}%`;
        }
        
        if (progressText) {
            const current = this.currentStep === -1 ? 0 : this.currentStep + 1;
            const total = this.totalSteps + 1;
            progressText.textContent = `${current} / ${total}`;
        }
    }
}

// ユーティリティ関数
const TechUtils = {
    // コードハイライト
    highlightCode: function(element) {
        if (!element) return;
        
        const keywords = ['fn', 'let', 'mut', 'const', 'static', 'unsafe', 'if', 'else', 'match', 'for', 'while', 'loop', 'return', 'impl', 'trait', 'struct', 'enum', 'use', 'mod', 'pub'];
        const types = ['i32', 'i64', 'u8', 'u32', 'u64', 'usize', 'char', 'bool', 'String', 'Vec', 'Option', 'Result'];
        
        let html = element.innerHTML;
        
        // キーワードのハイライト
        keywords.forEach(keyword => {
            const regex = new RegExp(`\\b${keyword}\\b`, 'g');
            html = html.replace(regex, `<span class="rust-keyword">${keyword}</span>`);
        });
        
        // 型のハイライト
        types.forEach(type => {
            const regex = new RegExp(`\\b${type}\\b`, 'g');
            html = html.replace(regex, `<span class="rust-type">${type}</span>`);
        });
        
        // 文字列のハイライト
        html = html.replace(/"([^"]*)"/g, '<span class="rust-string">"$1"</span>');
        html = html.replace(/'([^']*)'/g, '<span class="rust-string">\'$1\'</span>');
        
        // コメントのハイライト
        html = html.replace(/\/\/(.*)$/gm, '<span class="rust-comment">//$1</span>');
        
        element.innerHTML = html;
    },

    // 配列の3D表示を生成
    generate3DArray: function(containerId, data, rows = 16, cols = 8) {
        const container = document.getElementById(containerId);
        if (!container) return;
        
        container.innerHTML = '';
        
        for (let row = 0; row < rows; row++) {
            for (let col = 0; col < cols; col++) {
                const cell = document.createElement('div');
                cell.className = 'cell';
                
                if (data[row] && data[row][col] === '*') {
                    cell.classList.add('filled');
                    cell.textContent = '*';
                } else {
                    cell.textContent = ' ';
                }
                
                container.appendChild(cell);
            }
        }
    },

    // パフォーマンスチャートを生成
    generatePerformanceChart: function(containerId, data) {
        const container = document.getElementById(containerId);
        if (!container) return;
        
        container.innerHTML = '';
        
        data.forEach(item => {
            const bar = document.createElement('div');
            bar.className = `bar ${item.fast ? 'fast' : ''}`;
            bar.style.height = `${item.height}px`;
            bar.innerHTML = `${item.label}<br>${item.complexity}`;
            container.appendChild(bar);
        });
    },

    // スムーズスクロール
    smoothScrollTo: function(elementId) {
        const element = document.getElementById(elementId);
        if (element) {
            element.scrollIntoView({ 
                behavior: 'smooth', 
                block: 'start' 
            });
        }
    },

    // ローディングアニメーション
    showLoading: function(containerId) {
        const container = document.getElementById(containerId);
        if (container) {
            container.innerHTML = '<div class="loading">読み込み中...</div>';
        }
    },

    // エラー表示
    showError: function(containerId, message) {
        const container = document.getElementById(containerId);
        if (container) {
            container.innerHTML = `<div class="error">エラー: ${message}</div>`;
        }
    }
};

// ページ読み込み時の初期化
document.addEventListener('DOMContentLoaded', function() {
    // 技術解説ページの初期化
    if (document.querySelector('.step')) {
        window.techPage = new TechExplanationPage();
    }
    
    // コードブロックのハイライト
    document.querySelectorAll('.code-block').forEach(block => {
        TechUtils.highlightCode(block);
    });
    
    // 外部リンクを新しいタブで開く
    document.querySelectorAll('a[href^="http"]').forEach(link => {
        link.target = '_blank';
        link.rel = 'noopener noreferrer';
    });
});

// エクスポート（モジュール使用時）
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { TechExplanationPage, TechUtils };
}
