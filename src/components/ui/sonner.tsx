import { Toaster as Sonner } from "sonner";

type ToasterProps = React.ComponentProps<typeof Sonner>;

const Toaster = ({ ...props }: ToasterProps) => {
	return (
		<Sonner
			theme="light"
			className="toaster group"
			toastOptions={{
				classNames: {
					toast:
						"group toast group-[.toaster]:bg-card group-[.toaster]:text-card-foreground group-[.toaster]:border-0 group-[.toaster]:shadow-none",
					description: "group-[.toast]:text-card-foreground group-[.toast]:opacity-80",
					actionButton: "group-[.toast]:bg-background group-[.toast]:text-foreground",
					cancelButton:
						"group-[.toast]:bg-background group-[.toast]:text-foreground group-[.toast]:opacity-80",
					success: "group-[.toast]:bg-card group-[.toast]:text-card-foreground",
					error: "group-[.toast]:bg-destructive group-[.toast]:text-destructive-foreground",
					warning: "group-[.toast]:bg-card group-[.toast]:text-card-foreground",
					info: "group-[.toast]:bg-card group-[.toast]:text-card-foreground",
				},
			}}
			{...props}
		/>
	);
};

export { Toaster };
