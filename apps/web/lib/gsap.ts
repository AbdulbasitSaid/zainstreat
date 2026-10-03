import { gsap } from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { CustomEase } from "gsap/CustomEase";
import { useGSAP } from "@gsap/react";

gsap.registerPlugin(ScrollTrigger, CustomEase, useGSAP);

// Matches --ease-out-expo in globals.css so GSAP tweens use the same curve
// as the rest of the site's CSS transitions.
CustomEase.create("outExpo", "0.22, 1, 0.36, 1");

export { gsap, ScrollTrigger, useGSAP };
