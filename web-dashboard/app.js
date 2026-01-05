/**
 * Compass Blockchain Web Dashboard
 * JavaScript Application Logic
 */

// Configuration
const CONFIG = {
    rpcEndpoint: 'http://34.45.156.0:9000',
    pollInterval: 5000,  // 5 seconds
    maxBlocks: 20
};

// State
let state = {
    connected: false,
    currentPage: 'dashboard',
    walletAddress: null,
    chainHeight: 0
};

// ===== RPC Client =====
class CompassRPC {
    constructor(endpoint) {
        this.endpoint = endpoint;
        this.requestId = 0;
    }

    async call(method, params = {}) {
        try {
            const response = await fetch(this.endpoint, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    jsonrpc: '2.0',
                    method: method,
                    params: params,
                    id: ++this.requestId
                })
            });

            const data = await response.json();

            if (data.error) {
                throw new Error(data.error.message);
            }

            return data.result;
        } catch (error) {
            console.error(`RPC Error (${method}):`, error);
            throw error;
        }
    }

    // Convenience methods
    async getChainHeight() {
        return this.call('getChainHeight');
    }

    async getLatestBlocks(count = 10) {
        return this.call('getLatestBlocks', { count });
    }

    async getBlock(height) {
        return this.call('getBlock', { height });
    }

    async getBalance(walletId, asset) {
        return this.call('getBalance', { wallet_id: walletId, asset });
    }

    async getNodeInfo() {
        return this.call('getNodeInfo');
    }

    async getPeers() {
        return this.call('getPeers');
    }

    async getOraclePrices() {
        return this.call('getOraclePrices');
    }

    async getAllNFTs() {
        return this.call('getAllNFTs');
    }

    async getPaperTradingStats() {
        return this.call('getPaperTradingStats');
    }

    async getPaperTradeHistory() {
        return this.call('getPaperTradeHistory');
    }

    async getPortfolioSummary() {
        return this.call('getPortfolioSummary');
    }

    async getLatestSignal(ticker) {
        return this.call('getLatestSignal', { ticker });
    }

    async getAccountBalances(address) {
        return this.call('getAccountBalances', { address });
    }

    async submitTransaction(params) {
        return this.call('submitTransaction', params);
    }

    async trainModel(ticker) {
        return this.call('trainModel', { ticker });
    }

    async mintModelNFT(ticker, modelId, owner) {
        return this.call('mintModelNFT', { ticker, model_id: modelId, owner });
    }

    async getModelEpochStats(ticker, modelId, owner) {
        return this.call('getModelEpochStats', { ticker, model_id: modelId, owner });
    }

    async getMyModels(owner) {
        return this.call('getMyModels', { owner });
    }
}

// Initialize RPC client
let rpc = new CompassRPC(CONFIG.rpcEndpoint);

// ===== Page Navigation =====
function initNavigation() {
    document.querySelectorAll('.nav-item').forEach(item => {
        item.addEventListener('click', () => {
            const page = item.dataset.page;
            navigateTo(page);
        });
    });
}

function navigateTo(page) {
    // Update nav items
    document.querySelectorAll('.nav-item').forEach(item => {
        item.classList.toggle('active', item.dataset.page === page);
    });

    // Update pages
    document.querySelectorAll('.page').forEach(p => {
        p.classList.toggle('active', p.id === `page-${page}`);
    });

    // Update title
    const titles = {
        dashboard: 'Dashboard',
        wallet: 'Wallet',
        explorer: 'Block Explorer',
        trading: 'Paper Trading',
        models: 'AI Models',
        settings: 'Settings'
    };
    document.getElementById('pageTitle').textContent = titles[page] || page;

    state.currentPage = page;

    // Load page-specific data
    loadPageData(page);
}

async function loadPageData(page) {
    switch (page) {
        case 'dashboard':
            await Promise.all([
                refreshDashboard(),
                refreshBlocks(),
                refreshPrices()
            ]);
            break;
        case 'wallet':
            if (state.walletAddress) {
                await loadWalletBalances();
            }
            break;
        case 'explorer':
            await refreshExplorerBlocks();
            break;
        case 'trading':
            await Promise.all([
                loadTradingStats(),
                loadTradeHistory(),
                loadSignals()
            ]);
            break;
        case 'models':
            await Promise.all([
                refreshModelProgress(),
                refreshMyModels(),
                refreshModels()
            ]);
            break;
        case 'settings':
            await loadNodeInfo();
            break;
    }
}

// ===== Dashboard Functions =====
async function refreshDashboard() {
    try {
        const [heightData, peersData] = await Promise.all([
            rpc.getChainHeight(),
            rpc.getPeers()
        ]);

        state.chainHeight = heightData.height;

        document.getElementById('chainHeight').textContent =
            heightData.height.toLocaleString();
        document.getElementById('peerCount').textContent =
            peersData.peers?.length || 0;

        // Update connection status
        setConnectionStatus(true);

    } catch (error) {
        console.error('Dashboard refresh error:', error);
        setConnectionStatus(false);
    }
}

async function refreshBlocks() {
    const tbody = document.getElementById('blocksTableBody');

    try {
        const blocks = await rpc.getLatestBlocks(CONFIG.maxBlocks);

        if (!blocks || blocks.length === 0) {
            tbody.innerHTML = '<tr><td colspan="5" class="loading">No blocks found</td></tr>';
            return;
        }

        tbody.innerHTML = blocks.map(block => `
            <tr>
                <td><strong>${block.index}</strong></td>
                <td><code>${truncateHash(block.hash)}</code></td>
                <td><span class="badge badge-${getBlockTypeClass(block.block_type)}">${getBlockTypeName(block.block_type)}</span></td>
                <td><code>${truncateHash(block.proposer)}</code></td>
                <td>${formatTimestamp(block.timestamp)}</td>
            </tr>
        `).join('');

        // Update latest block time
        if (blocks[0]) {
            document.getElementById('latestBlockTime').textContent =
                formatTimestamp(blocks[0].timestamp);
        }

    } catch (error) {
        console.error('Blocks refresh error:', error);
        tbody.innerHTML = '<tr><td colspan="5" class="loading">Error loading blocks</td></tr>';
    }
}

async function refreshPrices() {
    try {
        const prices = await rpc.getOraclePrices();

        if (prices) {
            for (const [ticker, data] of Object.entries(prices)) {
                const tickerLower = ticker.toLowerCase().replace('usdt', '');
                const priceEl = document.getElementById(`price-${tickerLower}`);
                const signalEl = document.getElementById(`signal-${tickerLower}`);

                if (priceEl && data.price) {
                    priceEl.textContent = `$${formatNumber(data.price)}`;
                }

                if (signalEl && data.signal) {
                    signalEl.textContent = data.signal;
                    signalEl.className = `price-signal ${data.signal.toLowerCase()}`;
                }
            }
        }
    } catch (error) {
        console.error('Prices refresh error:', error);
    }
}

// ===== Wallet Functions =====
async function loadWallet() {
    const address = document.getElementById('walletAddressInput').value.trim();
    if (!address) {
        showToast('Please enter a wallet address', 'error');
        return;
    }

    state.walletAddress = address;
    localStorage.setItem('compass_wallet_address', address);
    await loadWalletBalances();

    // Refresh model progress if on that page
    if (state.currentPage === 'models') {
        refreshModelProgress();
    }
}

async function loadWalletBalances() {
    if (!state.walletAddress) return;

    try {
        const [compass, compute] = await Promise.all([
            rpc.getBalance(state.walletAddress, 'COMPASS'),
            rpc.getBalance(state.walletAddress, 'COMPUTE')
        ]);

        document.getElementById('balanceCompass').textContent =
            formatNumber(compass.balance / 1000000); // Assuming 6 decimals
        document.getElementById('balanceCompute').textContent =
            formatNumber(compute.balance);

        // Update header preview
        document.querySelector('.wallet-balance').textContent =
            `${formatNumber(compass.balance / 1000000)} COMPASS`;

    } catch (error) {
        console.error('Balance load error:', error);
        showToast('Failed to load balances', 'error');
    }
}

async function submitTransfer(event) {
    event.preventDefault();

    const to = document.getElementById('transferTo').value;
    const amount = parseFloat(document.getElementById('transferAmount').value);
    const asset = document.getElementById('transferAsset').value;

    if (!state.walletAddress) {
        showToast('Please load a wallet first', 'error');
        return;
    }

    try {
        const result = await rpc.submitTransaction({
            from: state.walletAddress,
            to: to,
            amount: Math.floor(amount * 1000000), // Convert to smallest unit
            asset: asset
        });

        showToast(`Transaction submitted: ${result.tx_hash.slice(0, 16)}...`, 'success');

        // Refresh balances after transfer
        setTimeout(loadWalletBalances, 2000);

    } catch (error) {
        showToast(`Transfer failed: ${error.message}`, 'error');
    }
}

function showCreateWallet() {
    // Generate a simple random address for demo
    const randomAddress = 'cmp_' + Array.from({ length: 40 }, () =>
        '0123456789abcdef'[Math.floor(Math.random() * 16)]
    ).join('');

    document.getElementById('walletAddressInput').value = randomAddress;
    showToast('New wallet address generated (demo mode)', 'success');
}

// ===== Explorer Functions =====
async function refreshExplorerBlocks() {
    const tbody = document.getElementById('explorerBlocksBody');

    try {
        const blocks = await rpc.getLatestBlocks(50);

        tbody.innerHTML = blocks.map(block => `
            <tr onclick="showBlockDetails(${block.index})" style="cursor: pointer;">
                <td><strong>${block.index}</strong></td>
                <td><code>${truncateHash(block.hash)}</code></td>
                <td>${getBlockTypeName(block.block_type)}</td>
                <td>${formatTimestamp(block.timestamp)}</td>
            </tr>
        `).join('');

    } catch (error) {
        tbody.innerHTML = '<tr><td colspan="4" class="loading">Error loading blocks</td></tr>';
    }
}

async function searchBlock() {
    const query = document.getElementById('explorerSearch').value.trim();
    if (!query) return;

    try {
        let block;
        if (/^\d+$/.test(query)) {
            // Search by height
            block = await rpc.getBlock(parseInt(query));
        } else {
            // TODO: Search by hash
            showToast('Hash search not yet implemented', 'error');
            return;
        }

        showBlockDetails(block);

    } catch (error) {
        showToast('Block not found', 'error');
    }
}

function showBlockDetails(blockOrHeight) {
    // If number, fetch block first
    if (typeof blockOrHeight === 'number') {
        rpc.getBlock(blockOrHeight).then(block => {
            displayBlockDetails(block);
        });
    } else {
        displayBlockDetails(blockOrHeight);
    }
}

function displayBlockDetails(block) {
    const container = document.getElementById('blockDetails');
    container.classList.remove('hidden');

    container.innerHTML = `
        <h3>Block #${block.index}</h3>
        <div class="info-row"><span>Hash</span><span>${block.hash}</span></div>
        <div class="info-row"><span>Previous Hash</span><span>${block.prev_hash}</span></div>
        <div class="info-row"><span>Type</span><span>${getBlockTypeName(block.block_type)}</span></div>
        <div class="info-row"><span>Proposer</span><span>${block.proposer}</span></div>
        <div class="info-row"><span>Timestamp</span><span>${new Date(block.timestamp).toLocaleString()}</span></div>
        <div class="info-row"><span>Signature</span><span>${truncateHash(block.signature_hex)}</span></div>
    `;
}

// ===== Trading Functions =====
async function loadTradingStats() {
    try {
        const [stats, portfolio] = await Promise.all([
            rpc.getPaperTradingStats(),
            rpc.getPortfolioSummary()
        ]);

        if (stats) {
            document.getElementById('totalTrades').textContent = stats.total_trades || 0;
            document.getElementById('winRate').textContent =
                (stats.win_rate ? (stats.win_rate * 100).toFixed(1) : 0) + '%';

            const pnl = stats.total_pnl || 0;
            const pnlEl = document.getElementById('totalPnl');
            pnlEl.textContent = `$${formatNumber(pnl)}`;
            pnlEl.className = `pnl ${pnl >= 0 ? 'positive' : 'negative'}`;
        }

        if (portfolio) {
            document.getElementById('portfolioValue').textContent =
                `$${formatNumber(portfolio.total_value || 0)}`;
        }

    } catch (error) {
        console.error('Trading stats error:', error);
    }
}

async function loadTradeHistory() {
    const tbody = document.getElementById('tradeHistoryBody');

    try {
        const history = await rpc.getPaperTradeHistory();

        if (!history || history.length === 0) {
            tbody.innerHTML = '<tr><td colspan="6" class="loading">No trades yet</td></tr>';
            return;
        }

        tbody.innerHTML = history.slice(0, 20).map(trade => `
            <tr>
                <td>${trade.ticker}</td>
                <td class="${trade.side?.toLowerCase()}">${trade.side}</td>
                <td>$${formatNumber(trade.entry_price)}</td>
                <td>$${formatNumber(trade.exit_price || '--')}</td>
                <td class="pnl ${trade.pnl >= 0 ? 'positive' : 'negative'}">
                    $${formatNumber(trade.pnl || 0)}
                </td>
                <td>${formatTimestamp(trade.timestamp)}</td>
            </tr>
        `).join('');

    } catch (error) {
        tbody.innerHTML = '<tr><td colspan="6" class="loading">Error loading history</td></tr>';
    }
}

async function loadSignals() {
    const container = document.getElementById('signalsList');
    const tickers = ['BTCUSDT', 'ETHUSDT', 'SOLUSDT', 'LTCUSDT'];

    try {
        const signals = await Promise.all(
            tickers.map(async ticker => {
                try {
                    return await rpc.getLatestSignal(ticker);
                } catch {
                    return null;
                }
            })
        );

        container.innerHTML = signals.filter(Boolean).map(signal => `
            <div class="signal-item">
                <span class="signal-ticker">${signal.ticker}</span>
                <span class="price-signal ${signal.signal?.toLowerCase()}">${signal.signal}</span>
                <span class="signal-price">$${formatNumber(signal.price)}</span>
            </div>
        `).join('') || '<div class="loading">No signals available</div>';

    } catch (error) {
        container.innerHTML = '<div class="loading">Error loading signals</div>';
    }
}

// ===== AI Models Functions =====
async function startTraining(ticker) {
    if (!state.walletAddress) {
        showToast('Please load a wallet first', 'error');
        navigateTo('wallet');
        return;
    }

    try {
        const result = await rpc.trainModel(ticker);
        showToast(`Training started for ${ticker}`, 'success');

        // Wait a bit and refresh progress
        setTimeout(refreshModelProgress, 2000);
    } catch (error) {
        showToast(`Failed to start training: ${error.message}`, 'error');
    }
}

async function refreshModelProgress() {
    const container = document.getElementById('modelProgressContainer');
    if (!state.walletAddress) {
        container.innerHTML = '<div class="loading">Load a wallet to see your model progress...</div>';
        return;
    }

    try {
        // Models we want to track
        const tickers = ['BTC', 'ETH', 'SOL', 'LTC'];
        const owner = state.walletAddress;

        let html = '';
        let foundAny = false;

        for (const ticker of tickers) {
            const tickerFull = `${ticker}USDT`;
            // Attempt to get stats for the model (assuming standard ID for now or fetching from MyModels)
            try {
                const modelId = `${tickerFull}_1h`;
                const stats = await rpc.getModelEpochStats(tickerFull, modelId, owner);

                if (stats) {
                    foundAny = true;
                    const accuracy = stats.total_predictions > 0
                        ? (stats.total_correct / stats.total_predictions)
                        : 0;
                    const progress = stats.predictions_in_epoch / stats.config.predictions_per_epoch;
                    const isMintable = stats.epochs_completed >= (stats.config.mint_at_epoch || 10) &&
                        accuracy >= stats.config.min_accuracy_to_mint;

                    html += `
                        <div class="progress-item">
                            <div class="progress-header">
                                <span class="progress-ticker">${tickerFull}</span>
                                <span class="progress-status ${isMintable ? 'ready' : 'training'}">
                                    ${isMintable ? 'Ready to Mint' : 'In Progress'}
                                </span>
                            </div>
                            <div class="progress-bar-container">
                                <div class="progress-bar" style="width: ${progress * 100}%"></div>
                            </div>
                            <div class="progress-info">
                                <span>Epoch: ${stats.current_epoch} (${stats.predictions_in_epoch}/${stats.config.predictions_per_epoch})</span>
                                <span>Accuracy: ${(accuracy * 100).toFixed(1)}%</span>
                            </div>
                            <div class="progress-actions">
                                ${isMintable ? `
                                    <button class="btn btn-primary" onclick="window.onMintClick('${tickerFull}', '${modelId}')">
                                        Mint Model NFT
                                    </button>
                                ` : `
                                    <span class="card-hint">Requirements: ${stats.config.mint_at_epoch || 10} epochs, ${(stats.config.min_accuracy_to_mint * 100).toFixed(0)}% accuracy</span>
                                `}
                            </div>
                        </div>
                    `;
                }
            } catch (e) {
                // Model might not exist yet for this user
                continue;
            }
        }

        if (!foundAny) {
            container.innerHTML = '<div class="loading">No active training/models found for this wallet. Click "Train" to start!</div>';
        } else {
            container.innerHTML = html;
        }

    } catch (error) {
        console.error('Progress refresh error:', error);
        container.innerHTML = '<div class="loading">Error loading progress</div>';
    }
}

async function onMintClick(ticker, modelId) {
    if (!state.walletAddress) return;

    try {
        showToast(`Minting NFT for ${ticker}...`, 'info');
        const result = await rpc.mintModelNFT(ticker, modelId, state.walletAddress);
        showToast(`Successfully minted! Tx: ${result.tx_hash.slice(0, 16)}...`, 'success');

        // Refresh everything
        setTimeout(() => {
            refreshModelProgress();
            refreshModels();
        }, 2000);

    } catch (error) {
        showToast(`Minting failed: ${error.message}`, 'error');
    }
}

async function refreshMyModels() {
    const grid = document.getElementById('myModelsGrid');
    if (!state.walletAddress) {
        grid.innerHTML = '<div class="loading">Load a wallet to see models you own...</div>';
        return;
    }

    try {
        const nfts = await rpc.getMyModels(state.walletAddress);

        if (!nfts || nfts.length === 0) {
            grid.innerHTML = `
                <div class="model-card placeholder">
                    <div class="model-icon">🤖</div>
                    <span>You don't own any models yet. Mint one above!</span>
                </div>
            `;
            return;
        }

        grid.innerHTML = renderModelCards(nfts);

    } catch (error) {
        console.error('My models refresh error:', error);
        grid.innerHTML = '<div class="loading">Error loading your models</div>';
    }
}

// Helper to render model cards (reused for Marketplace and My Models)
function renderModelCards(nfts) {
    return nfts.map(nft => `
        <div class="model-card">
            <div class="model-header">
                <span class="model-icon">🤖</span>
                <h3 class="model-name">${nft.name || nft.token_id || 'AI Model'}</h3>
            </div>
            <div class="model-stats">
                <div class="model-stat">
                    <span class="model-stat-value">${(nft.accuracy * 100).toFixed(1)}%</span>
                    <span class="model-stat-label">Accuracy</span>
                </div>
                <div class="model-stat">
                    <span class="model-stat-value">${(nft.win_rate * 100).toFixed(1)}%</span>
                    <span class="model-stat-label">Win Rate</span>
                </div>
                <div class="model-stat">
                    <span class="model-stat-value">${nft.total_predictions || 0}</span>
                    <span class="model-stat-label">Predictions</span>
                </div>
                <div class="model-stat">
                    <span class="model-stat-value">${nft.generation || 1}</span>
                    <span class="model-stat-label">Gen</span>
                </div>
            </div>
            <div class="model-footer">
                <span class="model-owner">Owner: ${truncateHash(nft.current_owner)}</span>
                ${nft.mint_price ? `<span class="model-price">${formatNumber(nft.mint_price)} COMPASS</span>` : ''}
            </div>
        </div>
    `).join('');
}

async function refreshModels() {
    const grid = document.getElementById('modelsGrid');

    try {
        const nfts = await rpc.getAllNFTs();

        if (!nfts || nfts.length === 0) {
            grid.innerHTML = `
                <div class="model-card placeholder">
                    <div class="model-icon">🤖</div>
                    <span>No models minted yet</span>
                </div>
            `;
            return;
        }

        grid.innerHTML = renderModelCards(nfts);

    } catch (error) {
        grid.innerHTML = `
            <div class="model-card placeholder">
                <div class="model-icon">❌</div>
                <span>Error loading models</span>
            </div>
        `;
    }
}

// ===== Settings Functions =====
async function loadNodeInfo() {
    try {
        const info = await rpc.getNodeInfo();

        document.getElementById('nodeVersion').textContent = info.version || '--';
        document.getElementById('nodeHeight').textContent = info.height || '--';
        document.getElementById('nodeHeadHash').textContent =
            truncateHash(info.head_hash) || '--';

    } catch (error) {
        console.error('Node info error:', error);
    }
}

function updateEndpoint() {
    const endpoint = document.getElementById('rpcEndpoint').value.trim();
    if (!endpoint) return;

    CONFIG.rpcEndpoint = endpoint;
    rpc = new CompassRPC(endpoint);

    showToast('Endpoint updated, reconnecting...', 'success');

    // Try to connect
    refreshDashboard();
}

// ===== Utility Functions =====
function setConnectionStatus(connected) {
    state.connected = connected;
    const indicator = document.getElementById('nodeStatusIndicator');
    const text = document.getElementById('nodeStatusText');

    indicator.className = `status-indicator ${connected ? 'connected' : 'disconnected'}`;
    text.textContent = connected ? 'Connected' : 'Disconnected';
}

function truncateHash(hash) {
    if (!hash) return '--';
    if (hash.length <= 16) return hash;
    return `${hash.slice(0, 8)}...${hash.slice(-6)}`;
}

function formatNumber(num) {
    if (num === undefined || num === null) return '--';
    return Number(num).toLocaleString(undefined, {
        minimumFractionDigits: 0,
        maximumFractionDigits: 2
    });
}

function formatTimestamp(ts) {
    if (!ts) return '--';
    const date = new Date(ts);
    return date.toLocaleTimeString();
}

function getBlockTypeName(blockType) {
    if (!blockType) return 'Unknown';
    if (typeof blockType === 'string') return blockType;

    // Handle object format
    const type = Object.keys(blockType)[0];
    return type || 'Unknown';
}

function getBlockTypeClass(blockType) {
    const name = getBlockTypeName(blockType).toLowerCase();
    if (name.includes('poh')) return 'primary';
    if (name.includes('transfer')) return 'success';
    if (name.includes('mint')) return 'warning';
    if (name.includes('genesis')) return 'info';
    return 'default';
}

function showToast(message, type = 'info') {
    const container = document.getElementById('toastContainer');
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.innerHTML = `
        <span>${type === 'success' ? '✓' : type === 'error' ? '✗' : 'ℹ'}</span>
        <span>${message}</span>
    `;
    container.appendChild(toast);

    setTimeout(() => {
        toast.remove();
    }, 4000);
}

function openModal(title, content) {
    document.getElementById('modalTitle').textContent = title;
    document.getElementById('modalBody').innerHTML = content;
    document.getElementById('modalOverlay').classList.add('active');
}

function closeModal() {
    document.getElementById('modalOverlay').classList.remove('active');
}

// ===== Polling =====
let pollTimer = null;

function startPolling() {
    if (pollTimer) return;

    pollTimer = setInterval(async () => {
        if (state.currentPage === 'dashboard') {
            await refreshDashboard();
        }
    }, CONFIG.pollInterval);
}

function stopPolling() {
    if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
    }
}

// ===== Initialize =====
document.addEventListener('DOMContentLoaded', () => {
    console.log('🧭 Compass Dashboard Initializing...');

    // Setup navigation
    initNavigation();

    // Initial load
    const savedAddress = localStorage.getItem('compass_wallet_address');
    if (savedAddress) {
        state.walletAddress = savedAddress;
        document.getElementById('walletAddressInput').value = savedAddress;
        loadWalletBalances();
    }

    navigateTo('dashboard');

    // Start polling
    startPolling();

    // Close modal on overlay click
    document.getElementById('modalOverlay').addEventListener('click', (e) => {
        if (e.target.id === 'modalOverlay') {
            closeModal();
        }
    });

    console.log('✅ Dashboard Ready');
});

// Expose functions globally for onclick handlers
window.refreshBlocks = refreshBlocks;
window.loadWallet = loadWallet;
window.submitTransfer = submitTransfer;
window.showCreateWallet = showCreateWallet;
window.searchBlock = searchBlock;
window.showBlockDetails = showBlockDetails;
window.refreshModels = refreshModels;
window.updateEndpoint = updateEndpoint;
window.closeModal = closeModal;
window.startTraining = startTraining;
window.refreshModelProgress = refreshModelProgress;
window.refreshMyModels = refreshMyModels;
window.onMintClick = onMintClick;
