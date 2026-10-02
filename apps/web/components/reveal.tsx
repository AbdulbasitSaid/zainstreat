"use client";

import { motion, useReducedMotion, type Variants } from "motion/react";
import type { ReactNode } from "react";

const fadeUp: Variants = {
  hidden: { opacity: 0, y: 16 },
  show: { opacity: 1, y: 0 },
};

export function Reveal({
  children,
  stagger = false,
}: {
  children: ReactNode;
  stagger?: boolean;
}) {
  const shouldReduceMotion = useReducedMotion();

  return (
    <motion.div
      initial={shouldReduceMotion ? false : "hidden"}
      whileInView="show"
      viewport={{ once: true, amount: 0.2 }}
      variants={fadeUp}
      transition={{
        duration: shouldReduceMotion ? 0 : 0.5,
        ease: [0.22, 1, 0.36, 1],
        staggerChildren: stagger ? 0.08 : 0,
      }}
    >
      {children}
    </motion.div>
  );
}
