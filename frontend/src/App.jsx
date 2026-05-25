import { useState, useEffect, useRef } from 'react'
import './index.css'

const API_BASE = 'http://localhost:8080'

function App() {
  const [conversations, setConversations] = useState([])
  const [currentConversationId, setCurrentConversationId] = useState(null)
  const [messages, setMessages] = useState([
    { id: 1, role: 'bot', content: '您好！我是 Hermes AI，请问有什么可以帮助您的？', time: new Date().toLocaleTimeString() }
  ])
  const [input, setInput] = useState('')
  const [isLoading, setIsLoading] = useState(false)

  useEffect(() => {
    const saved = localStorage.getItem('hermes_conversations')
    if (saved) {
      const convs = JSON.parse(saved)
      if (convs.length > 0) {
        setConversations(convs)
        setCurrentConversationId(convs[0].id)
        setMessages(convs[0].messages)
      }
    } else {
      const defaultConv = {
        id: 'conv-1',
        title: '新对话',
        messages: [{ id: 1, role: 'bot', content: '您好！我是 Hermes AI，请问有什么可以帮助您的？', time: new Date().toLocaleTimeString() }],
        createdAt: Date.now()
      }
      setConversations([defaultConv])
      setCurrentConversationId(defaultConv.id)
      localStorage.setItem('hermes_conversations', JSON.stringify([defaultConv]))
    }
  }, [])

  const saveConversations = (newConversations) => {
    setConversations(newConversations)
    localStorage.setItem('hermes_conversations', JSON.stringify(newConversations))
  }

  const createNewConversation = () => {
    const newConv = {
      id: `conv-${Date.now()}`,
      title: '新对话',
      messages: [{ id: 1, role: 'bot', content: '您好！我是 Hermes AI，请问有什么可以帮助您的？', time: new Date().toLocaleTimeString() }],
      createdAt: Date.now()
    }
    const newConversations = [newConv, ...conversations]
    saveConversations(newConversations)
    setCurrentConversationId(newConv.id)
    setMessages(newConv.messages)
    setInput('')
  }

  const selectConversation = (convId) => {
    const conv = conversations.find(c => c.id === convId)
    if (conv) {
      setCurrentConversationId(convId)
      setMessages(conv.messages)
      setInput('')
    }
  }

  const updateConversationMessages = (convId, newMessages) => {
    const newConversations = conversations.map(conv => 
      conv.id === convId 
        ? { ...conv, messages: newMessages, updatedAt: Date.now() }
        : conv
    )
    saveConversations(newConversations)
  }

  const updateConversationTitle = (convId, title) => {
    const newConversations = conversations.map(conv => 
      conv.id === convId ? { ...conv, title } : conv
    )
    saveConversations(newConversations)
  }

  const deleteConversation = (convId) => {
    if (conversations.length <= 1) return
    const newConversations = conversations.filter(c => c.id !== convId)
    saveConversations(newConversations)
    if (currentConversationId === convId) {
      const firstConv = newConversations[0]
      setCurrentConversationId(firstConv.id)
      setMessages(firstConv.messages)
    }
  }

  const truncateText = (text, maxLength = 40) => {
    if (text.length <= maxLength) return text
    return text.substring(0, maxLength) + '...'
  }

  const formatDate = (timestamp) => {
    const date = new Date(timestamp)
    const now = new Date()
    const diff = now - date
    if (diff < 60000) return '刚刚'
    if (diff < 3600000) return `${Math.floor(diff / 60000)}分钟前`
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}小时前`
    if (diff < 604800000) return `${Math.floor(diff / 86400000)}天前`
    return date.toLocaleDateString()
  }

  const handleSend = async () => {
    if (!input.trim() || isLoading) return
    
    setIsLoading(true)
    const userMsg = { id: Date.now(), role: 'user', content: input.trim(), time: new Date().toLocaleTimeString() }
    const newMessages = [...messages, userMsg]
    setMessages(newMessages)
    updateConversationMessages(currentConversationId, newMessages)
    setInput('')

    if (messages.length === 1 && messages[0].role === 'bot') {
      updateConversationTitle(currentConversationId, truncateText(input.trim()))
    }

    try {
      const response = await fetch(`${API_BASE}/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ message: input.trim() })
      })
      
      if (response.ok) {
        const data = await response.json()
        const botMsg = { id: Date.now() + 1, role: 'bot', content: data.response, time: new Date().toLocaleTimeString() }
        const updatedMessages = [...newMessages, botMsg]
        setMessages(updatedMessages)
        updateConversationMessages(currentConversationId, updatedMessages)
      } else {
        const errorMsg = { id: Date.now() + 1, role: 'bot', content: `错误：服务器返回 ${response.status}`, time: new Date().toLocaleTimeString() }
        const updatedMessages = [...newMessages, errorMsg]
        setMessages(updatedMessages)
        updateConversationMessages(currentConversationId, updatedMessages)
      }
    } catch (error) {
      console.error('Fetch error:', error)
      const errorMsg = { id: Date.now() + 1, role: 'bot', content: `网络错误：${error.message}`, time: new Date().toLocaleTimeString() }
      const updatedMessages = [...newMessages, errorMsg]
      setMessages(updatedMessages)
      updateConversationMessages(currentConversationId, updatedMessages)
    } finally {
      setIsLoading(false)
    }
  }

  const messagesEndRef = useRef(null)
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  return (
    <div className="app-container">
      <div className="sidebar">
        <div className="sidebar-header">
          <button className="new-chat-btn" onClick={createNewConversation}>
            <span>+</span>
            <span>新建对话</span>
          </button>
        </div>

        <div className="sidebar-nav">
          <div className="nav-item">
            <span className="nav-icon">📁</span>
            <span>我的空间</span>
          </div>
          <div className="nav-item">
            <span className="nav-icon">🤖</span>
            <span>智能体</span>
          </div>
        </div>

        <div className="sidebar-section">
          <div className="section-header">
            <span className="section-title">对话分组</span>
            <span className="section-add">+</span>
          </div>
          <div className="nav-item">
            <span className="nav-icon">📋</span>
            <span>新分组</span>
          </div>
        </div>

        <div className="sidebar-section">
          <div className="section-header">
            <span className="section-title">最近对话</span>
            <span className="section-more">···</span>
          </div>
          <div className="conversations-list">
            {conversations.map((conv, index) => (
              <div 
                key={conv.id} 
                className={`conversation-item ${currentConversationId === conv.id ? 'active' : ''}`}
                onClick={() => selectConversation(conv.id)}
              >
                <div className="conv-icon">
                  {index === 0 && currentConversationId === conv.id ? '🦅' : '💬'}
                </div>
                <div className="conv-info">
                  <div className="conv-title">{conv.title}</div>
                  <div className="conv-preview">
                    {conv.messages.length > 0 ? 
                      truncateText(conv.messages[conv.messages.length - 1].content) :
                      '暂无消息'
                    }
                  </div>
                </div>
                <div className="conv-meta">
                  <div className="conv-time">{formatDate(conv.createdAt)}</div>
                  <button 
                    className="delete-conv-btn" 
                    onClick={(e) => { e.stopPropagation(); deleteConversation(conv.id); }}
                    title="删除对话"
                  >
                    ✕
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="sidebar-footer">
          <div className="user-info">
            <div className="user-avatar">👤</div>
            <div className="user-details">
              <div className="user-name">Hermes User</div>
              <div className="user-role">普通用户</div>
            </div>
          </div>
        </div>
      </div>

      <div className="main-content">
        <div className="chat-header">
          <div className="chat-title-row">
            <select className="model-select">
              <option value="qwen3.5">Qwen3.5-千问</option>
            </select>
            <div className="chat-actions">
              <button className="action-btn" title="新版本">🔄</button>
              <button className="action-btn" title="分享">📤</button>
              <button className="action-btn" title="设置">⚙️</button>
            </div>
          </div>
        </div>

        <div className="messages-container">
          {messages.map(msg => (
            <div key={msg.id} className={`message ${msg.role}`}>
              <div className="message-avatar">
                {msg.role === 'bot' ? '🦅' : '👤'}
              </div>
              <div className="message-bubble">
                <div className="message-content">{msg.content}</div>
                <div className="message-time">{msg.time}</div>
              </div>
            </div>
          ))}
          {isLoading && (
            <div className="message bot">
              <div className="message-avatar">🦅</div>
              <div className="message-bubble">
                <div className="loading">
                  <span className="loading-dot"></span>
                  <span className="loading-dot"></span>
                  <span className="loading-dot"></span>
                </div>
              </div>
            </div>
          )}
          <div ref={messagesEndRef} />
        </div>

        <div className="input-container">
          <div className="input-tools">
            <button className="tool-btn" title="附件">📎</button>
            <button className="tool-btn" title="任务助理">📋</button>
            <button className="tool-btn" title="思考">💭</button>
            <button className="tool-btn" title="研究">🔍</button>
            <button className="tool-btn" title="更多">⋮</button>
          </div>
          <textarea
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault()
                handleSend()
              }
            }}
            placeholder="向千问提问"
            disabled={isLoading}
          />
          <button className="send-btn" onClick={handleSend} disabled={!input.trim() || isLoading}>
            {isLoading ? '⏳' : '➤'}
          </button>
        </div>
        <div className="footer-hint">
          内容由AI生成，可能不准确，请注意核实
        </div>
      </div>
    </div>
  )
}

export default App
