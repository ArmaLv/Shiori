import React, { useState, useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { LucideIcon, X } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface RadialMenuItem {
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  destructive?: boolean;
}

interface RadialMenuProps {
  children: React.ReactNode;
  items: RadialMenuItem[];
}

function polarToCartesian(centerX: number, centerY: number, radius: number, angleInDegrees: number) {
  const angleInRadians = (angleInDegrees - 90) * Math.PI / 180.0;
  return {
    x: centerX + (radius * Math.cos(angleInRadians)),
    y: centerY + (radius * Math.sin(angleInRadians))
  };
}

function getPieSlice(cx: number, cy: number, r: number, ir: number, startAngle: number, endAngle: number) {
  const gap = 2; // 2 degree gap for aesthetic separation
  const start = polarToCartesian(cx, cy, r, endAngle - gap/2);
  const end = polarToCartesian(cx, cy, r, startAngle + gap/2);
  const innerStart = polarToCartesian(cx, cy, ir, endAngle - gap/2);
  const innerEnd = polarToCartesian(cx, cy, ir, startAngle + gap/2);
  
  const largeArcFlag = endAngle - startAngle <= 180 ? "0" : "1";
  
  return [
    "M", start.x, start.y, 
    "A", r, r, 0, largeArcFlag, 0, end.x, end.y,
    "L", innerEnd.x, innerEnd.y,
    "A", ir, ir, 0, largeArcFlag, 1, innerStart.x, innerStart.y,
    "Z"
  ].join(" ");
}

export function RadialMenu({ children, items }: RadialMenuProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const touchTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  
  const setScreenCenterPosition = () => {
    setPosition({
      x: window.innerWidth / 2,
      y: window.innerHeight / 2
    });
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setScreenCenterPosition();
    setIsOpen(true);
  };

  const handleTouchStart = (e: React.TouchEvent) => {
    if (e.touches.length > 1) return;
    touchTimer.current = setTimeout(() => {
      setScreenCenterPosition();
      setIsOpen(true);
      if (typeof navigator !== 'undefined' && navigator.vibrate) {
        navigator.vibrate(50);
      }
    }, 500);
  };

  const clearTouchTimer = () => {
    if (touchTimer.current) clearTimeout(touchTimer.current);
  };

  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden';
      const handleClose = () => setIsOpen(false);
      window.addEventListener('scroll', handleClose, { capture: true });
      window.addEventListener('resize', handleClose);
      return () => {
        document.body.style.overflow = '';
        window.removeEventListener('scroll', handleClose, { capture: true });
        window.removeEventListener('resize', handleClose);
      };
    }
  }, [isOpen]);

  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  return (
    <>
      <div 
        ref={wrapperRef}
        onContextMenu={handleContextMenu} 
        onTouchStart={handleTouchStart}
        onTouchEnd={clearTouchTimer}
        onTouchMove={clearTouchTimer}
        onTouchCancel={clearTouchTimer}
        className="w-full h-full"
      >
        {children}
      </div>

      {mounted && document.body && createPortal(
        <AnimatePresence>
          {isOpen && (
            <div className="fixed inset-0 z-[100]" onContextMenu={(e) => e.preventDefault()}>
              <motion.div 
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="absolute inset-0 bg-black/60 dark:bg-black/80 pointer-events-auto" 
                onClick={() => setIsOpen(false)}
                onTouchEnd={(e) => {
                  e.preventDefault();
                  setIsOpen(false);
                }}
              />
              
              <div 
                className="absolute pointer-events-none"
                style={{ left: position.x, top: position.y }}
              >
                <motion.div
                  initial={{ scale: 0.3, opacity: 0, rotate: -20 }}
                  animate={{ scale: 1, opacity: 1, rotate: 0 }}
                  exit={{ scale: 0.3, opacity: 0, rotate: 20 }}
                  transition={{ type: 'spring', stiffness: 350, damping: 25 }}
                  className="absolute left-0 top-0 pointer-events-none will-change-transform"
                >
                  {/* SVG Pie Slices */}
                  <svg width="340" height="340" className="absolute pointer-events-none" style={{ left: -170, top: -170 }}>
                    {items.map((item, i) => {
                      const anglePerSlice = 360 / items.length;
                      const startAngle = i * anglePerSlice - anglePerSlice / 2;
                      const endAngle = startAngle + anglePerSlice;
                      const pathData = getPieSlice(170, 170, 160, 50, startAngle, endAngle);
                      const isHovered = hoveredIndex === i;
                      
                      return (
                        <path
                          key={`slice-${item.label}`}
                          d={pathData}
                          className="pointer-events-auto cursor-pointer transition-colors duration-200"
                          style={{
                            fill: isHovered 
                              ? (item.destructive ? "hsl(var(--destructive)/0.9)" : "hsl(var(--primary)/0.9)") 
                              : "hsl(var(--card)/0.95)",
                            stroke: "hsl(var(--border)/0.5)",
                            strokeWidth: 1.5
                          }}
                          onTouchStart={() => setHoveredIndex(i)}
                          onTouchEnd={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                            setHoveredIndex(null);
                            setIsOpen(false);
                            item.onClick();
                          }}
                          onMouseEnter={() => setHoveredIndex(i)}
                          onMouseLeave={() => setHoveredIndex(null)}
                          onClick={(e) => {
                            e.stopPropagation();
                            setIsOpen(false);
                            item.onClick();
                          }}
                        />
                      );
                    })}
                  </svg>
                  
                  {/* HTML Content Overlays */}
                  {items.map((item, i) => {
                    const anglePerSlice = 360 / items.length;
                    const sliceCenterAngle = i * anglePerSlice;
                    const r = (160 + 50) / 2; // Midpoint radius = 105
                    const center = polarToCartesian(0, 0, r, sliceCenterAngle);
                    const isHovered = hoveredIndex === i;

                    return (
                      <div 
                        key={`content-${item.label}`}
                        className="absolute pointer-events-none flex flex-col items-center justify-center gap-1 transition-transform duration-200"
                        style={{ 
                          left: center.x, 
                          top: center.y,
                          transform: `translate(-50%, -50%) scale(${isHovered ? 1.1 : 1})`
                        }}
                      >
                        <item.icon 
                          size={24} 
                          className={cn(
                            "transition-colors",
                            isHovered 
                              ? (item.destructive ? "text-destructive-foreground" : "text-primary-foreground") 
                              : (item.destructive ? "text-destructive" : "text-foreground")
                          )} 
                        />
                        <span className={cn(
                          "text-[11px] font-semibold text-center leading-tight transition-colors",
                          isHovered 
                            ? (item.destructive ? "text-destructive-foreground" : "text-primary-foreground") 
                            : "text-muted-foreground"
                        )}>
                          {item.label}
                        </span>
                      </div>
                    )
                  })}
                </motion.div>
                
                {/* Center Cancel Button */}
                <motion.div 
                  initial={{ scale: 0, opacity: 0 }}
                  animate={{ scale: 1, opacity: 1 }}
                  exit={{ scale: 0, opacity: 0 }}
                  className="absolute w-16 h-16 rounded-full bg-card border border-border shadow-2xl flex items-center justify-center text-muted-foreground z-20 pointer-events-auto cursor-pointer hover:bg-muted hover:text-foreground hover:scale-110 transition-all duration-200 will-change-transform"
                  style={{ left: -32, top: -32 }}
                  onClick={(e) => {
                    e.stopPropagation();
                    setIsOpen(false);
                  }}
                  onTouchEnd={(e) => {
                    e.stopPropagation();
                    e.preventDefault();
                    setIsOpen(false);
                  }}
                >
                  <X size={28} />
                </motion.div>
              </div>
            </div>
          )}
        </AnimatePresence>,
        document.body
      )}
    </>
  );
}
