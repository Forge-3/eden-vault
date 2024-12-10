// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";

abstract contract TransactionThrottler is Ownable {
    bool internal _restrictionActive;

    uint256 private constant _delayBetweenTx = 30;
    uint256 private _maxTransferAmount;
    uint256 private _maxWalletAmount;

    mapping(address => bool) private _isWhitelisted;
    mapping(address => bool) private _isUnthrottled;
    mapping(address => uint256) private _previousTx;

    event RestrictionActiveChanged(bool active);
    event MaxTransferAmountChanged(uint256 maxTransferAmount);
    event MarkedWhitelisted(address indexed account, bool isWhitelisted);
    event MarkedUnthrottled(address indexed account, bool isUnthrottled);

    error ZeroAddressIsProhibited();
    error MaxTransferAmountExceeded();
    error TransactionProtectionIntervalLimit();

    constructor(address initialOwner) Ownable(initialOwner) {}

    function setMaxTransferAmount(uint256 amount) external virtual onlyOwner {
        _maxTransferAmount = amount;
        emit MaxTransferAmountChanged(_maxTransferAmount);
    }

    function setRestrictionActive(bool active) external onlyOwner {
        _restrictionActive = active;
        emit RestrictionActiveChanged(_restrictionActive);
    }

    function unthrottleAccount(
        address account,
        bool unthrottled
    ) public virtual onlyOwner {
        if (account == address(0)) {
            revert ZeroAddressIsProhibited();
        }
        _isUnthrottled[account] = unthrottled;
        emit MarkedUnthrottled(account, unthrottled);
    }

    function isUnthrottled(address account) external view returns (bool) {
        return _isUnthrottled[account];
    }

    function whitelistAccount(
        address account,
        bool whitelisted
    ) public virtual onlyOwner {
        if (account == address(0)) {
            revert ZeroAddressIsProhibited();
        }
        _isWhitelisted[account] = whitelisted;
        emit MarkedWhitelisted(account, whitelisted);
    }

    function isWhitelisted(address account) external view returns (bool) {
        return _isWhitelisted[account];
    }

    modifier transactionThrottler(
        address sender,
        address recipient,
        uint256 amount
    ) virtual {
        if (
            _restrictionActive &&
            !_isUnthrottled[recipient] &&
            !_isUnthrottled[sender]
        ) {
            if (_maxTransferAmount > 0) {
                if (amount > _maxTransferAmount) {
                    revert MaxTransferAmountExceeded();
                }
            }

            if (!_isWhitelisted[recipient]) {
                if (
                    _previousTx[recipient] + _delayBetweenTx > block.timestamp
                ) {
                    revert TransactionProtectionIntervalLimit();
                }
                _previousTx[recipient] = block.timestamp;
            }

            if (!_isWhitelisted[sender]) {
                if (_previousTx[sender] + _delayBetweenTx > block.timestamp) {
                    revert TransactionProtectionIntervalLimit();
                }
                _previousTx[sender] = block.timestamp;
            }
        }
        _;
    }
}

interface ILssController {
    function beforeTransfer(
        address sender,
        address recipient,
        uint256 amount
    ) external;

    function beforeTransferFrom(
        address msgSender,
        address sender,
        address recipient,
        uint256 amount
    ) external;

    function beforeApprove(
        address sender,
        address spender,
        uint256 amount
    ) external;

    function beforeIncreaseAllowance(
        address msgSender,
        address spender,
        uint256 addedValue
    ) external;

    function beforeDecreaseAllowance(
        address msgSender,
        address spender,
        uint256 subtractedValue
    ) external;

    function beforeBurn(address account, uint256 amount) external;
}

contract EdenTokenLL is TransactionThrottler {
    mapping(address => uint256) private _balances;
    mapping(address => mapping(address => uint256)) private _allowances;
    uint256 private _totalSupply;
    string private _name;
    string private _symbol;

    address public recoveryAdmin;
    address private recoveryAdminCandidate;
    bytes32 private recoveryAdminKeyHash;
    address public admin;
    uint256 public timelockPeriod;
    uint256 public losslessTurnOffTimestamp;
    bool public isLosslessOn = true;
    ILssController public lossless;

    constructor(
        uint256 totalSupply_,
        string memory name_,
        string memory symbol_,
        address admin_,
        address recoveryAdmin_,
        uint256 timelockPeriod_,
        address lossless_
    ) TransactionThrottler(msg.sender) {
        _mint(_msgSender(), totalSupply_);
        _name = name_;
        _symbol = symbol_;
        admin = admin_;
        recoveryAdmin = recoveryAdmin_;
        recoveryAdminCandidate = address(0);
        recoveryAdminKeyHash = "";
        timelockPeriod = timelockPeriod_;
        losslessTurnOffTimestamp = 0;
        lossless = ILssController(lossless_);
    }

    event Transfer(address indexed _from, address indexed _to, uint256 _value);
    event Approval(
        address indexed _owner,
        address indexed _spender,
        uint256 _value
    );
    event NewAdmin(address indexed _newAdmin);
    event NewRecoveryAdminProposal(address indexed _candidate);
    event NewRecoveryAdmin(address indexed _newAdmin);
    event LosslessTurnOffProposal(uint256 _turnOffDate);
    event LosslessOff();
    event LosslessOn();

    // --- LOSSLESS modifiers ---

    modifier lssAprove(address spender, uint256 amount) {
        if (isLosslessOn) {
            lossless.beforeApprove(_msgSender(), spender, amount);
        }
        _;
    }

    modifier lssTransfer(address recipient, uint256 amount) {
        if (isLosslessOn) {
            lossless.beforeTransfer(_msgSender(), recipient, amount);
        }
        _;
    }

    modifier lssTransferFrom(
        address sender,
        address recipient,
        uint256 amount
    ) {
        if (isLosslessOn) {
            lossless.beforeTransferFrom(
                _msgSender(),
                sender,
                recipient,
                amount
            );
        }
        _;
    }

    modifier lssIncreaseAllowance(address spender, uint256 addedValue) {
        if (isLosslessOn) {
            lossless.beforeIncreaseAllowance(_msgSender(), spender, addedValue);
        }
        _;
    }

    modifier lssDecreaseAllowance(address spender, uint256 subtractedValue) {
        if (isLosslessOn) {
            lossless.beforeDecreaseAllowance(
                _msgSender(),
                spender,
                subtractedValue
            );
        }
        _;
    }

    modifier onlyRecoveryAdmin() {
        require(
            _msgSender() == recoveryAdmin,
            "LERC20: Must be recovery admin"
        );
        _;
    }

    modifier lssBurn(address account, uint256 amount) {
        if (isLosslessOn) {
            lossless.beforeBurn(account, amount);
        }
        _;
    }

    // --- LOSSLESS management ---
    function transferOutBlacklistedFunds(address[] calldata from) external {
        require(
            _msgSender() == address(lossless),
            "LERC20: Only lossless contract"
        );
        require(isLosslessOn, "LERC20: Lossless is off");

        uint256 fromLength = from.length;
        uint256 totalAmount = 0;

        for (uint256 i = 0; i < fromLength; i++) {
            address fromAddress = from[i];
            uint256 fromBalance = _balances[fromAddress];
            _balances[fromAddress] = 0;
            totalAmount += fromBalance;
            emit Transfer(fromAddress, address(lossless), fromBalance);
        }

        _balances[address(lossless)] += totalAmount;
    }

    function setLosslessAdmin(address newAdmin) external onlyRecoveryAdmin {
        require(newAdmin != admin, "LERC20: Cannot set same address");
        emit NewAdmin(newAdmin);
        admin = newAdmin;
    }

    function transferRecoveryAdminOwnership(
        address candidate,
        bytes32 keyHash
    ) external onlyRecoveryAdmin {
        recoveryAdminCandidate = candidate;
        recoveryAdminKeyHash = keyHash;
        emit NewRecoveryAdminProposal(candidate);
    }

    function acceptRecoveryAdminOwnership(bytes memory key) external {
        require(
            _msgSender() == recoveryAdminCandidate,
            "LERC20: Must be canditate"
        );
        require(keccak256(key) == recoveryAdminKeyHash, "LERC20: Invalid key");
        emit NewRecoveryAdmin(recoveryAdminCandidate);
        recoveryAdmin = recoveryAdminCandidate;
        recoveryAdminCandidate = address(0);
    }

    function proposeLosslessTurnOff() external onlyRecoveryAdmin {
        require(
            losslessTurnOffTimestamp == 0,
            "LERC20: TurnOff already proposed"
        );
        require(isLosslessOn, "LERC20: Lossless already off");
        losslessTurnOffTimestamp = block.timestamp + timelockPeriod;
        emit LosslessTurnOffProposal(losslessTurnOffTimestamp);
    }

    function executeLosslessTurnOff() external onlyRecoveryAdmin {
        require(losslessTurnOffTimestamp != 0, "LERC20: TurnOff not proposed");
        require(
            losslessTurnOffTimestamp <= block.timestamp,
            "LERC20: Time lock in progress"
        );
        isLosslessOn = false;
        losslessTurnOffTimestamp = 0;
        emit LosslessOff();
    }

    function executeLosslessTurnOn() external onlyRecoveryAdmin {
        require(!isLosslessOn, "LERC20: Lossless already on");
        losslessTurnOffTimestamp = 0;
        isLosslessOn = true;
        emit LosslessOn();
    }

    function getAdmin() public view virtual returns (address) {
        return admin;
    }

    // --- ERC20 methods ---

    function name() public view virtual returns (string memory) {
        return _name;
    }

    function symbol() public view virtual returns (string memory) {
        return _symbol;
    }

    function decimals() public view virtual returns (uint8) {
        return 18;
    }

    function totalSupply() public view virtual returns (uint256) {
        return _totalSupply;
    }

    function balanceOf(address account) public view virtual returns (uint256) {
        return _balances[account];
    }

    function transfer(
        address recipient,
        uint256 amount
    ) public virtual lssTransfer(recipient, amount) returns (bool) {
        _transfer(_msgSender(), recipient, amount);
        return true;
    }

    function allowance(
        address owner,
        address spender
    ) public view virtual returns (uint256) {
        return _allowances[owner][spender];
    }

    function approve(
        address spender,
        uint256 amount
    ) public virtual lssAprove(spender, amount) returns (bool) {
        _approve(_msgSender(), spender, amount);
        return true;
    }

    function transferFrom(
        address sender,
        address recipient,
        uint256 amount
    ) public virtual lssTransferFrom(sender, recipient, amount) returns (bool) {
        uint256 currentAllowance = _allowances[sender][_msgSender()];
        require(
            currentAllowance >= amount,
            "LERC20: transfer amount exceeds allowance"
        );
        _transfer(sender, recipient, amount);

        _approve(sender, _msgSender(), currentAllowance - amount);

        return true;
    }

    function increaseAllowance(
        address spender,
        uint256 addedValue
    ) public virtual lssIncreaseAllowance(spender, addedValue) returns (bool) {
        _approve(
            _msgSender(),
            spender,
            _allowances[_msgSender()][spender] + addedValue
        );
        return true;
    }

    function decreaseAllowance(
        address spender,
        uint256 subtractedValue
    )
        public
        virtual
        lssDecreaseAllowance(spender, subtractedValue)
        returns (bool)
    {
        uint256 currentAllowance = _allowances[_msgSender()][spender];
        require(
            currentAllowance >= subtractedValue,
            "LERC20: decreased allowance below zero"
        );
        _approve(_msgSender(), spender, currentAllowance - subtractedValue);

        return true;
    }

    function _transfer(
        address sender,
        address recipient,
        uint256 amount
    ) internal virtual transactionThrottler(sender, recipient, amount){
        require(sender != address(0), "LERC20: transfer from the zero address");

        uint256 senderBalance = _balances[sender];
        require(
            senderBalance >= amount,
            "LERC20: transfer amount exceeds balance"
        );
        _balances[sender] = senderBalance - amount;
        _balances[recipient] += amount;

        emit Transfer(sender, recipient, amount);
    }

    function _mint(address account, uint256 amount) internal virtual {
        require(account != address(0), "LERC20: mint to the zero address");

        _totalSupply += amount;

        // Cannot overflow because the sum of all user
        // balances can't exceed the max uint256 value.
        unchecked {
            _balances[account] += amount;
        }
        emit Transfer(address(0), account, amount);
    }

    function _burn(address account, uint256 amount) internal virtual {
        require(account != address(0), "ERC20: burn from the zero address");

        uint256 accountBalance = _balances[account];
        require(accountBalance >= amount, "ERC20: burn amount exceeds balance");
        unchecked {
            _balances[account] = accountBalance - amount;
        }
        _totalSupply -= amount;

        emit Transfer(account, address(0), amount);
    }

    function _approve(
        address owner,
        address spender,
        uint256 amount
    ) internal virtual {
        _allowances[owner][spender] = amount;
        emit Approval(owner, spender, amount);
    }

    function burn(uint256 amount) public virtual lssBurn(_msgSender(), amount) {
        _burn(_msgSender(), amount);
    }

    function burnFrom(
        address account,
        uint256 amount
    ) public virtual lssBurn(account, amount) {
        uint256 currentAllowance = allowance(account, _msgSender());
        require(
            currentAllowance >= amount,
            "ERC20: burn amount exceeds allowance"
        );
        unchecked {
            _approve(account, _msgSender(), currentAllowance - amount);
        }
        _burn(account, amount);
    }
}