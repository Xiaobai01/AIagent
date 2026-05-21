import { useState, useEffect, useRef } from 'react'
import './index.css'

const SKILLS = [
  { name: 'read_file', icon: '📄', label: '读取文件', desc: '读取文件内容', category: 'file' },
  { name: 'write_file', icon: '✏️', label: '写入文件', desc: '写入文件内容', category: 'file' },
  { name: 'list_directory', icon: '📁', label: '目录列表', desc: '列出目录内容', category: 'folder' },
  { name: 'http_get', icon: '🌐', label: 'HTTP 请求', desc: '发送 GET 请求', category: 'web' },
  { name: 'calculate', icon: '🧮', label: '计算器', desc: '数学计算', category: 'calc' },
]

const API_BASE = 'http://localhost:8080';

function App() {
  const [messages, setMessages] = useState([
    { id: 1, role: 'bot', content: '你好！我是你的 AI 助手。有什么我可以帮你的吗？', time: new Date().toLocaleTimeString() }
  ])
  const [input, setInput] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [stats, setStats] = useState({ short_term: 0, long_term: 0, skills: 5 })

  const handleSend = async () => {
    if (!input.trim() || isLoading) return

    const userMessage = {
      id: messages.length + 1,
      role: 'user',
      content: input.trim(),
      time: new Date().toLocaleTimeString()
    }

    setMessages(prev => [...prev, userMessage])
    setInput('')
    setIsLoading(true)

    try {
      const response = await fetch(`${API_BASE}/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ message: input.trim() })
      })

      if (response.ok) {
        const data = await response.json()
        const botMessage = {
          id: messages.length + 2,
          role: 'bot',
          content: data.response,
          time: new Date().toLocaleTimeString()
        }
        setMessages(prev => [...prev, botMessage])

        if (data.stats) {
          setStats(data.stats)
        }
      } else {
        const botMessage = {
          id: messages.length + 2,
          role: 'bot',
          content: '抱歉，我遇到了一些问题，请稍后重试。',
          time: new Date().toLocaleTimeString()
        }
        setMessages(prev => [...prev, botMessage])
      }
    } catch (error) {
      const botMessage = {
        id: messages.length + 2,
        role: 'bot',
        content: `连接错误: ${error.message}`,
        time: new Date().toLocaleTimeString()
      }
      setMessages(prev => [...prev, botMessage])
    } finally {
      setIsLoading(false)
    }
  }

  const handleKeyDown = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const handleSkillClick = (skill) => {
    setInput(`使用 ${skill.label} 工具`)
  }

  const handleClearMemory = async () => {
    try {
      await fetch(`${API_BASE}/clear`, { method: 'POST' })
      setStats(prev => ({ ...prev, short_term: 0, long_term: 0 }))
      alert('记忆已清空')
    } catch (error) {
      alert('清空失败')
    }
  }

  return (
    <div className="app-container">
      <Sidebar skills={SKILLS} stats={stats} onSkillClick={handleSkillClick} onClearMemory={handleClearMemory} />
      <MainContent
        messages={messages}
        input={input}
        setInput={setInput}
        isLoading={isLoading}
        onSend={handleSend}
        onKeyDown={handleKeyDown}
      />
    </div>
  )
}

function Sidebar({ skills, stats, onSkillClick, onClearMemory }) {
  return (
    <div className="sidebar">
      <div className="sidebar-header">
        <div className="logo-icon">🤖</div>
        <div className="logo-text">AI Agent</div>
      </div>

      <div className="sidebar-section">
        <div className="section-title">技能</div>
        <div className="skill-list">
          {skills.map((skill) => (
            <div
              key={skill.name}
              className="skill-item"
              onClick={() => onSkillClick(skill)}
            >
              <div className={`skill-icon ${skill.category}`}>{skill.icon}</div>
              <div>
                <div className="skill-name">{skill.label}</div>
                <div className="skill-desc">{skill.desc}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="sidebar-section">
        <div className="section-title">记忆状态</div>
        <div className="memory-stats">
          <div className="stat-item">
            <span className="stat-label">短期记忆</span>
            <span className="stat-value">{stats.short_term} 条</span>
          </div>
          <div className="stat-item">
            <span className="stat-label">长期记忆</span>
            <span className="stat-value">{stats.long_term} 条</span>
          </div>
          <div className="stat-item">
            <span className="stat-label">可用技能</span>
            <span className="stat-value">{stats.skills} 个</span>
          </div>
        </div>
      </div>

      <div className="sidebar-section" style={{ marginTop: 'auto' }}>
        <button className="btn btn-secondary" onClick={onClearMemory} style={{ width: '100%' }}>
          清空记忆
        </button>
      </div>
    </div>
  )
}

function MainContent({ messages, input, setInput, isLoading, onSend, onKeyDown }) {
  const messagesEndRef = useRef(null)

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  return (
    <div className="main-content">
      <Header />

      <div className="messages-container">
        {messages.map((msg) => (
          <Message key={msg.id} message={msg} />
        ))}

        {isLoading && (
          <div className="message message-bot">
            <div className="avatar avatar-bot">🤖</div>
            <div className="message-content">
              <div className="loading">
                <span>思考中</span>
                <div className="loading-dots">
                  <span className="loading-dot"></span>
                  <span className="loading-dot"></span>
                  <span className="loading-dot"></span>
                </div>
              </div>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      <div className="input-container">
        <div className="input-wrapper">
          <div className="textarea-wrapper">
            <textarea
              className="input-textarea"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={onKeyDown}
              placeholder="输入消息..."
              rows={1}
            />
          </div>
          <button
            className="send-btn"
            onClick={onSend}
            disabled={!input.trim() || isLoading}
          >
            →
          </button>
        </div>
      </div>
    </div>
  )
}

function Header() {
  return (
    <div className="header">
      <div className="header-left">
        <div className="agent-avatar">🤖</div>
        <div className="agent-info">
          <div className="agent-name">AI Assistant</div>
          <div className="agent-status">
            <span className="status-dot"></span>
            <span>在线</span>
          </div>
        </div>
      </div>
      <div className="header-right">
        <button className="btn btn-secondary">设置</button>
        <button className="btn btn-primary">新建对话</button>
      </div>
    </div>
  )
}

function Message({ message }) {
  return (
    <div className={`message message-${message.role}`}>
      <div className={`avatar avatar-${message.role}`}>
        {message.role === 'user' ? '👤' : '🤖'}
      </div>
      <div>
        <div className="message-content">{message.content}</div>
        <div className="message-time">{message.time}</div>
      </div>
    </div>
  )
}

export default App
